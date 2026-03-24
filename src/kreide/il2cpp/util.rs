use patternscan::scan_first_match;
use std::{borrow::Cow, cell::OnceCell, ffi::CStr};
use windows::{
    Win32::{
        Foundation::HMODULE,
        System::{
            ProcessStatus::{GetModuleInformation, MODULEINFO},
            Threading::GetCurrentProcess,
        },
    },
    core::s,
};

use crate::{UNITYPLAYER_HANDLE, prelude::*};

/// # SAFETY
/// This is guaranteed to be safe inside il2cpp runtime.
pub unsafe fn cstr_to_str(ptr: *const i8) -> Cow<'static, str> {
    unsafe { Cow::Borrowed(CStr::from_ptr(ptr).to_str().unwrap_unchecked()) }
}

unsafe fn unity_player_slice() -> &'static [u8] {
    static mut SLICE: OnceCell<&[u8]> = OnceCell::new();
    unsafe {
        SLICE.get_or_init(|| {
            let module = HMODULE(*UNITYPLAYER_HANDLE as _);
            let mut module_info = MODULEINFO {
                lpBaseOfDll: std::ptr::null_mut(),
                SizeOfImage: 0,
                EntryPoint: std::ptr::null_mut(),
            };
            GetModuleInformation(
                GetCurrentProcess(),
                module,
                &mut module_info,
                std::mem::size_of::<MODULEINFO>() as u32,
            )
            .unwrap();
            std::slice::from_raw_parts(
                module.0 as *const u8,
                module_info.SizeOfImage.try_into().unwrap(),
            )
        })
    }
}

/// returns relative address
pub unsafe fn scan_unity_player_section(pat: &str) -> Result<usize> {
    let mut slice = unsafe { unity_player_slice() };
    scan_first_match(&mut slice, pat)?
        .map(|address| {
            let slice = unsafe { unity_player_slice() };
            match slice.get(address) {
                // jmp sub_xxxxxxx
                Some(&0xE8) => {
                    let offset =
                        i32::from_le_bytes(slice[address + 1..address + 5].try_into().unwrap());
                    address + 5 + offset as usize
                }
                // mov REGISTER, [rip + offset] (0x48 0x8B 0x0D XXXXXXXX)
                Some(&0x48) if slice.get(address + 1) == Some(&0x8B) => {
                    let offset =
                        i32::from_le_bytes(slice[address + 3..address + 7].try_into().unwrap());
                    address + 7 + offset as usize
                }
                _ => address,
            }
        })
        .context("Could not find UnityPlayer RVA")
}

#[macro_export]
macro_rules! cs_class {
    ($class_name:expr) => {
        pub fn get_class() -> anyhow::Result<$crate::kreide::il2cpp::api::Il2CppClass> {
            use crate::prelude::*;
            $crate::kreide::il2cpp::get_cached_class($class_name).with_context(|| format!("no such class {}", stringify!($class_name)))
        }

        pub fn is_null(&self) -> bool {
            self.0 == 0
        }

        pub fn as_object(&self) -> Il2CppObject {
            Il2CppObject(self.0)
        }
    };
}

/// ```rust
/// cs_method!(pub set_target_framerate, "set_targetFrameRate", &["int"], (), (fps: i32));
/// ```
///
/// ```rust
/// cs_method!(pub to_string, "ToString", &[], Il2CppString, (), self);
/// ```
#[macro_export]
macro_rules! cs_method {
    (
        $vis:vis $fn_name:ident,
        $method_name:expr,
        $arg_types:expr,
        $ret:ty,
        ($($arg_name:ident : $arg_ty:ty),*)
    ) => {
        #[allow(warnings)]
        $vis fn $fn_name($($arg_name: $arg_ty),*) -> anyhow::Result<$ret> {
            use crate::prelude::*;

            unsafe {
                let arg_types: &[&str] = $arg_types;
                let class = Self::get_class()?;

                let method_info = class.find_method($method_name, arg_types).with_context(|| format!("no such method {} in {}", $method_name, class.byval_arg().name()))?;

                let func: extern "fastcall" fn($($arg_ty),*) -> $ret = std::mem::transmute(method_info.va());
                microseh::try_seh(||
                    func($($arg_name),*)
                )
                .map_err(|e|
                    anyhow::anyhow!(
                        "Runtime Exception! (static method call throws exception! type: {} method name: 0x{:X}): {:?}",
                        class.byval_arg().name(),
                        method_info.rva(),
                        e
                    )
                )
            }
        }
    };

    (
        $vis:vis $fn_name:ident,
        $method_name:expr,
        $arg_types:expr,
        $ret:ty,
        ($($arg_name:ident : $arg_ty:ty),*),
        self
    ) => {
        #[allow(warnings)]
        $vis fn $fn_name(&self, $($arg_name: $arg_ty),*) -> anyhow::Result<$ret> {
            use crate::prelude::*;

            unsafe {
                let arg_types: &[&str] = $arg_types;
                if self.0 == 0 {
                    return Err(anyhow::format_err!("object reference is not set to an instance of an object! method name: {}", $method_name));
                }

                let class = $crate::kreide::il2cpp::native::Il2CppObject(self.0).get_class();

                let method_info = class.find_method($method_name, arg_types).with_context(|| format!("no such method {} in {}", $method_name, class.byval_arg().name()))?;

                let func: extern "fastcall" fn(usize, $($arg_ty),*) -> $ret = std::mem::transmute(method_info.va());
                microseh::try_seh(||
                    func(self.0, $($arg_name),*)
                )
                .map_err(|e|
                    anyhow::anyhow!(
                        "Runtime Exception! (instance method call throws exception! type: {} method name: {}): {:?}",
                        class.byval_arg().name(),
                        method_info.name(),
                        e
                    )
                )
            }
        }
    };
}

///
/// ```rust
/// cs_field!(a, "static");
/// ```
///
/// ```rust
/// cs_field!(b, "static transform", |field| -> u32 { 1 });
/// ```
///
/// ```rust
/// cs_field!(c, "instance", self);
/// ```
///
/// ```rust
/// cs_field!(d, "instance transform", self, |field| -> u32 { field.unbox::<u32>() });
/// ```
#[macro_export]
macro_rules! cs_field {
    // static field Il2CppObject
    ($ident:ident, $name:literal) => {
        $crate::cs_field!(@internal, $ident, $name, static, |value| -> $crate::kreide::il2cpp::native::Il2CppObject { value });
    };

    // instance field Il2CppObject
    ($ident:ident, $name:literal, self) => {
        $crate::cs_field!(@internal, $ident, $name, instance, |value| -> $crate::kreide::il2cpp::native::Il2CppObject { value });
    };

    // static field custom
    ($ident:ident, $name:literal, |$param:ident| -> $ret_ty:ty $block:block) => {
        $crate::cs_field!(@internal, $ident, $name, static, |$param| -> $ret_ty $block);
    };

    // instance field custom
    ($ident:ident, $name:literal, self, |$param:ident| -> $ret_ty:ty $block:block) => {
        $crate::cs_field!(@internal, $ident, $name, instance, |$param| -> $ret_ty $block);
    };

    // static field
    (@internal, $ident:ident, $name:literal, static, |$param:ident| -> $ret_ty:ty $block:block) => {
        paste::paste! {
            pub fn $ident() -> anyhow::Result<$ret_ty> {
                use crate::prelude::*;

                let class = RuntimeType::from_class(Self::get_class()?);
                let field_info = class.get_field($name).with_context(|| format!("no such static field {} in type {}", $name, class.get_il2cpp_type().name()))?;

                let value = microseh::try_seh(|| {
                    field_info.get_value($crate::kreide::il2cpp::native::Il2CppObject::NULL).with_context(|| format!("field {} in {} is null", class.get_il2cpp_type().name(), $name))
                })
                .map_err(|e| {
                    anyhow::anyhow!(
                        "Runtime Exception! failed to get static field value. type: {} name: {}. {:?}",
                        class.get_il2cpp_type().name(),
                        field_info.name(),
                        e
                    )
                })?;

                let $param: $crate::kreide::il2cpp::native::Il2CppObject = value?;
                Ok($block)
            }
        }
    };

    // instance field
    (@internal, $ident:ident, $name:literal, instance, |$param:ident| -> $ret_ty:ty $block:block) => {
        paste::paste! {
            pub fn $ident(&self) -> anyhow::Result<$ret_ty> {
                use crate::prelude::*;

                if self.0 == 0 {
                    return Err(anyhow::format_err!(
                        "object reference is not set to an instance of an object! method name: {}",
                        $name
                    ));
                }

                let class = RuntimeType::from_class(self.as_object().get_class());
                let field_info = class.get_field($name).with_context(|| format!("no such static field {} in type {}", $name, class.get_il2cpp_type().name()))?;

                let value = microseh::try_seh(|| {
                    field_info.get_value($crate::kreide::il2cpp::native::Il2CppObject(self.0)).with_context(|| format!("field {} in {} is null", class.get_il2cpp_type().name(), $name))
                })
                .map_err(|e| {
                    anyhow::anyhow!(
                        "Runtime Exception! failed to get instance field value. type: {} name: {}. {:?}",
                        class.get_il2cpp_type().name(),
                        field_info.name(),
                        e
                    )
                })?;

                let $param: $crate::kreide::il2cpp::native::Il2CppObject = value?;
                Ok($block)
            }
        }
    };
}

#[macro_export]
macro_rules! cs_property {
    (
        $vis:vis $fn_name:ident,
        $getter_fn:expr,
        $ret:path,
        self
    ) => {
        paste::paste! {
           $crate::cs_method!(
                $vis [<get_ $fn_name>],
                $getter_fn,
                &[], $ret,
                (),
                self
            );
        }
    };

    (
        $vis:vis $fn_name:ident,
        $getter_fn:expr,
        enumtype $ret:ty,
        self
    ) => {
        $crate::cs_method!($fn_name, $getter_fn, &[], i32, (), self);

        paste::paste! {
            $vis fn [<get_ $fn_name>](&self) -> anyhow::Result<$ret> {
                let obj = self.$fn_name()?;
                #[allow(clippy::missing_transmute_annotations)]
                Ok(unsafe { std::mem::transmute(obj) })
            }
        }
    };
}

#[macro_export]
macro_rules! cs_dynamic_class {
    ($resolver_key:expr) => {
        pub fn get_class() -> anyhow::Result<$crate::kreide::il2cpp::api::Il2CppClass> {
            use crate::prelude::*;
            let real_name = $crate::kreide::resolver::get_dynamic_name($resolver_key);
            $crate::kreide::il2cpp::get_cached_class(&real_name).with_context(|| format!("Không tìm thấy class {}", real_name))
        }

        pub fn is_null(&self) -> bool {
            self.0 == 0
        }

        pub fn as_object(&self) -> Il2CppObject {
            Il2CppObject(self.0)
        }
    };
}

#[macro_export]
macro_rules! cs_dynamic_field {
    ($ident:ident, $resolver_key:expr, self, |$param:ident| -> $ret_ty:ty $block:block) => {
        paste::paste! {
            pub fn $ident(&self) -> anyhow::Result<$ret_ty> {
                use crate::prelude::*;

                if self.0 == 0 {
                    return Err(anyhow::format_err!("Đối tượng bị null khi gọi field."));
                }

                let real_field_name = $crate::kreide::resolver::get_dynamic_name($resolver_key);
                let class = RuntimeType::from_class(self.as_object().get_class());
                let field_info = class.get_field(&real_field_name)?;

                let value = microseh::try_seh(|| {
                    field_info.get_value($crate::kreide::il2cpp::native::Il2CppObject(self.0))
                })
                .map_err(|e| anyhow::anyhow!("SEH Error in dynamic field {}: {:?}", $resolver_key, e))??;

                let $param: $crate::kreide::il2cpp::native::Il2CppObject = value;
                Ok($block)
            }
        }
    };
}