use super::{native::Il2CppObject, util};
use crate::prelude::*;
use std::borrow::Cow;

macro_rules! il2cpp_api {
    ($index:expr, $name:ident($($arg_name:ident: $arg_type:ty),*) -> $ret_type:ty) => {
        #[allow(warnings)]
        pub fn $name($($arg_name: $arg_type,)*) -> $ret_type {
            unsafe {
                type FuncType = unsafe extern "fastcall" fn($($arg_type,)*) -> $ret_type;
                ::std::mem::transmute::<usize, FuncType>(
                    *((*crate::UNITYPLAYER_HANDLE + super::API_BASE_PTR + 8 * $index) as *const usize)
                )($($arg_name,)*)
           }
        }
    };
}

il2cpp_api!(22, il2cpp_assembly_get_image(assembly: Il2CppAssembly) -> Il2CppImage);
il2cpp_api!(35, il2cpp_class_get_methods(klass: Il2CppClass, iter: *const *const usize) -> Il2CppMethod);
il2cpp_api!(37, il2cpp_class_get_name(klass: Il2CppClass) -> *const i8);
il2cpp_api!(49, il2cpp_class_from_type(r#type: Il2CppType) -> Il2CppClass);
il2cpp_api!(63, il2cpp_domain_get() -> Il2CppDomain);
il2cpp_api!(65, il2cpp_domain_get_assemblies(domain: Il2CppDomain, size: *mut usize) -> *mut Il2CppAssembly);
il2cpp_api!(73, il2cpp_field_get_name(field: Il2CppField) -> *const i8);
il2cpp_api!(77, il2cpp_field_get_value_object(field: Il2CppField, obj: Il2CppObject) -> Il2CppObject);
il2cpp_api!(116, il2cpp_method_get_return_type(method: Il2CppMethod) -> Il2CppType);
il2cpp_api!(117, il2cpp_method_get_name(method: Il2CppMethod) -> *const i8);
il2cpp_api!(123, il2cpp_method_get_param_count(method: Il2CppMethod) -> u32);
il2cpp_api!(124, il2cpp_method_get_param(method: Il2CppMethod, index: u32) -> Il2CppType);
il2cpp_api!(154, il2cpp_thread_attach(domain: Il2CppDomain) -> usize);
il2cpp_api!(161, il2cpp_type_get_name(r#type: Il2CppType) -> *const i8);
il2cpp_api!(169, il2cpp_image_get_class_count(image: Il2CppImage) -> usize);
il2cpp_api!(170, il2cpp_image_get_class(image: Il2CppImage, index: usize) -> Il2CppClass);

#[repr(transparent)]
#[derive(Debug, Copy, Clone)]
pub struct Il2CppDomain(pub usize);

impl Il2CppDomain {
    pub fn assemblies(&self) -> Vec<Il2CppAssembly> {
        let mut count = 0;
        let assemblies = il2cpp_domain_get_assemblies(*self, &mut count);
        unsafe { std::slice::from_raw_parts(assemblies, count).to_vec() }
    }
}

#[repr(transparent)]
#[derive(Debug, Copy, Clone)]
pub struct Il2CppImage(pub usize);

impl Il2CppImage {
    pub fn class_count(&self) -> usize {
        il2cpp_image_get_class_count(*self)
    }

    pub fn classes(&self) -> Vec<Il2CppClass> {
        (0..self.class_count())
            .map(|index| il2cpp_image_get_class(*self, index))
            .collect()
    }
}

#[repr(transparent)]
#[derive(Debug, Copy, Clone)]
pub struct Il2CppAssembly(pub usize);

#[repr(transparent)]
#[derive(Debug, Copy, Clone)]
pub struct Il2CppClass(pub usize);

impl Il2CppClass {
    pub fn name(&self) -> Cow<'static, str> {
        unsafe { util::cstr_to_str(il2cpp_class_get_name(*self)) }
    }

    pub fn byval_arg(&self) -> Il2CppType {
        Il2CppType(self.0 + 128)
    }

    pub fn methods(&self) -> Vec<Il2CppMethod> {
        let iter = std::ptr::null();
        let mut result = Vec::new();
        loop {
            let method = il2cpp_class_get_methods(*self, &iter);
            if method.0 == 0 {
                break;
            }
            result.push(method)
        }
        result
    }

    // pub fn find_method_by_name(&self, name: &str) -> Option<Il2CppMethod> {
    //     self.methods()
    //         .into_iter()
    //         .find(|&method| method.name() == name)
    // }

    pub fn find_method(&self, name: &str, arg_types: &[&str]) -> Result<Il2CppMethod> {
        let qualified_name = format!("{}::{}", self.name(), name);
        let mut last_error: Option<anyhow::Error> = None;

        for method in self.methods().iter().filter(|m| m.name() == name) {
            let count = method.args_cnt() as usize;

            if count == arg_types.len() {
                let mut fail = false;
                let mut mismatched_index = 0;

                for (i, arg_type) in arg_types.iter().enumerate() {
                    if *arg_type != method.arg_type_formatted(i as u32) {
                        fail = true;
                        mismatched_index = i;
                        break;
                    }
                }

                if !fail {
                    return Ok(*method);
                } else {
                    last_error = Some(anyhow!(
                        "Method {} arg {} should be {}",
                        qualified_name,
                        arg_types[mismatched_index],
                        method.arg_type_formatted(mismatched_index as u32)
                    ));
                }
            } else {
                last_error = Some(anyhow!(
                    "Method {} contains {} args and not {} args",
                    qualified_name,
                    count,
                    arg_types.len()
                ));
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow!("No method named {}", qualified_name)))
    }

    pub fn find_method_full(
        &self,
        name: &str,
        arg_types: &[&str],
        ret_type: &str,
    ) -> Result<Il2CppMethod> {
        let qualified_name = format!("{}::{}", self.name(), name);
        let mut last_error: Option<anyhow::Error> = None;
        let mut name_matched = false;

        for method in self.methods().iter().filter(|m| m.name() == name) {
            name_matched = true;

            let ret = il2cpp_method_get_return_type(*method);
            let count = method.args_cnt() as usize;

            if count != arg_types.len() {
                last_error = Some(anyhow!(
                    "Method {} contains {} args and not {} args",
                    qualified_name,
                    count,
                    arg_types.len()
                ));
                continue;
            }

            if ret.formatted_name() != ret_type {
                last_error = Some(anyhow!(
                    "Method {} returns {} and not {}",
                    qualified_name,
                    ret.formatted_name(),
                    ret_type
                ));
                continue;
            }

            let mut fail = false;
            let mut mismatched_index = 0;
            for (i, arg_type) in arg_types.iter().enumerate() {
                if *arg_type != method.arg_type_formatted(i as u32) {
                    fail = true;
                    mismatched_index = i;
                    break;
                }
            }

            if fail {
                last_error = Some(anyhow!(
                    "Method {} arg {} should be {}",
                    qualified_name,
                    arg_types[mismatched_index],
                    method.arg_type_formatted(mismatched_index as u32)
                ));
                continue;
            }

            return Ok(*method);
        }

        if !name_matched {
            Err(anyhow!("No method name matched {}", qualified_name))
        } else {
            Err(last_error
                .unwrap_or_else(|| anyhow!("No overload of {} matched signature", qualified_name)))
        }
    }
}

#[repr(transparent)]
#[derive(Debug, Copy, Clone)]
pub struct Il2CppType(pub usize);

impl Il2CppType {
    pub fn name(&self) -> Cow<'static, str> {
        unsafe { util::cstr_to_str(il2cpp_type_get_name(*self)) }
    }

    pub fn formatted_name(&self) -> String {
        let name = self.name();

        (match name.as_ref() {
            "System.Int32" => "int",
            "System.UInt32" => "uint",
            "System.Int16" => "short",
            "System.UInt16" => "ushort",
            "System.Int64" => "long",
            "System.UInt64" => "ulong",
            "System.Byte" => "byte",
            "System.SByte" => "sbyte",
            "System.Boolean" => "bool",
            "System.Single" => "float",
            "System.Double" => "double",
            "System.String" => "string",
            "System.Char" => "char",
            "System.Object" => "object",
            "System.Void" => "void",
            "System.Decimal" => "decimal",
            "System.DateTime" => "DateTime",
            other => other,
        })
        .to_string()
    }

    pub fn class(&self) -> Il2CppClass {
        il2cpp_class_from_type(*self)
    }
}

#[repr(transparent)]
#[derive(Debug, Copy, Clone)]
pub struct Il2CppMethod(pub usize);

impl Il2CppMethod {
    pub fn name(&self) -> Cow<'static, str> {
        unsafe { util::cstr_to_str(il2cpp_method_get_name(*self)) }
    }

    pub fn class(&self) -> Il2CppClass {
        unsafe { *((self.0) as *const Il2CppClass) }
    }

    pub fn va(&self) -> usize {
        unsafe { *((self.0 + 8) as *const usize) }
    }

    pub fn rva(&self) -> usize {
        let va = self.va();
        if va == 0 {
            return 0;
        }
        self.va() - *crate::GAMEASSEMBLY_HANDLE
    }

    pub fn args_cnt(&self) -> u32 {
        il2cpp_method_get_param_count(*self)
    }

    pub fn arg(&self, i: u32) -> Il2CppType {
        il2cpp_method_get_param(*self, i)
    }

    pub fn arg_type_formatted(&self, i: u32) -> String {
        self.arg(i).formatted_name()
    }

    pub fn format_params(&self) -> String {
        use std::fmt::Write;
        let param_count = il2cpp_method_get_param_count(*self);
        let name = self.name();
        let mut out = String::with_capacity(0);

        let _ = write!(out, "{name}(");
        for param_index in 0..param_count {
            let param = il2cpp_method_get_param(*self, param_index);
            let _ = write!(out, "{}", param.class().byval_arg().formatted_name());

            if param_index + 1 < param_count {
                let _ = write!(out, ",");
            }
        }
        let _ = write!(out, ")");

        out
    }
}

#[repr(transparent)]
#[derive(Debug, Copy, Clone)]
pub struct Il2CppField(pub usize);

impl Il2CppField {
    pub fn name(&self) -> Cow<'static, str> {
        unsafe { util::cstr_to_str(il2cpp_field_get_name(*self)) }
    }

    pub fn get_value(&self, instance: Il2CppObject) -> Result<Il2CppObject> {
        let value = il2cpp_field_get_value_object(*self, instance);
        if value.0 == 0 {
            Err(anyhow!("Could not get value of field {}", self.name()))
        } else {
            Ok(value)
        }
    }
}
