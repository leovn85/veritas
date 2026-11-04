use darling::FromMeta;

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    FnArg, Ident, ItemEnum, ItemFn, ItemStruct, LitStr, Pat, Token,
    parse::{Parse, ParseStream},
    parse_macro_input,
    token::Paren,
};

#[derive(Debug, FromMeta)]
#[darling(derive_syn_parse)]
struct MethodMacroArgs {
    name: LitStr,
    #[darling(default)]
    args: darling::util::SpannedValue<Vec<LitStr>>,
    #[darling(default)]
    extension: bool,
    #[darling(default)]
    ret_type: Option<LitStr>,
    #[darling(default)]
    value_type: bool,
}

/// Generates an IL2CPP method wrapper that resolves the target method,
/// then calls it. The generated function returns `Result<_, Il2CppError>`.
#[proc_macro_attribute]
pub fn il2cpp_method(args: TokenStream, input: TokenStream) -> TokenStream {
    let macro_args: MethodMacroArgs = match syn::parse(args) {
        Ok(parsed_args) => parsed_args,
        Err(parse_error) => {
            return parse_error.to_compile_error().into();
        }
    };

    let function_def: ItemFn = syn::parse_macro_input!(input as ItemFn);
    let function_name = &function_def.sig.ident;

    let function_signature = &function_def.sig;
    let function_vis = &function_def.vis;

    let il2cpp_method_name = &macro_args.name;
    let il2cpp_method_args: Vec<_> = macro_args.args.iter().collect();
    let il2cpp_method_arg_count = il2cpp_method_args.len();
    let il2cpp_method_ret_type = macro_args.ret_type.as_ref();

    let make_method_lookup = |receiver: proc_macro2::TokenStream| {
        if let Some(ret_type) = il2cpp_method_ret_type {
            quote! {
                #receiver.find_method_with_ret_type(
                    #il2cpp_method_name,
                    &[#(#il2cpp_method_args),*],
                    #ret_type
                )
            }
        } else {
            quote! {
                #receiver.find_method(#il2cpp_method_name, &[#(#il2cpp_method_args),*])
            }
        }
    };
    let class_method_lookup = make_method_lookup(quote! { class });
    let parent_method_lookup = make_method_lookup(quote! { parent });

    let is_static_method = !function_def
        .sig
        .inputs
        .iter()
        .any(|arg: &FnArg| matches!(arg, FnArg::Receiver(_)));

    let (parameter_names, parameter_types) = extract_function_parameters(&function_def);

    if parameter_names.len() != il2cpp_method_arg_count {
        let error_message = format!(
            "[il2cpp_method] ERROR: Parameter mismatch for '{}' - Rust function has {} parameters but IL2CPP args has {} arguments",
            function_name,
            parameter_names.len(),
            il2cpp_method_arg_count
        );
        let compile_error =
            syn::Error::new_spanned(function_signature, error_message).to_compile_error();

        return TokenStream::from(compile_error);
    }

    let method_call = if is_static_method {
        quote! { method(#(#parameter_names),*) }
    } else if macro_args.extension {
        quote! { method(core::ptr::null(), #(#parameter_names),*) }
    } else if macro_args.value_type {
        quote! { method(self, #(#parameter_names),*) }
    } else {
        quote! { method(self.0, #(#parameter_names),*) }
    };

    let extern_fn_params = if is_static_method {
        quote! { #(#parameter_types),* }
    } else if macro_args.value_type {
        let self_type = match function_def.sig.inputs.iter().next() {
            Some(FnArg::Receiver(receiver)) => {
                if receiver.reference.is_some() {
                    quote! { &Self }
                } else {
                    quote! { Self }
                }
            }
            _ => quote! { Self },
        };
        quote! { #self_type, #(#parameter_types),* }
    } else {
        quote! { *const std::ffi::c_void, #(#parameter_types),* }
    };

    // QUAN TRỌNG: Null check phải luôn chạy ở mọi lần gọi hàm (nằm ngoài OnceLock)
    let runtime_null_check = if !is_static_method && !macro_args.extension && !macro_args.value_type {
        quote! {
            if self.0.is_null() {
                log_method_error(format_args!(
                    "[il2cpp_method] Null self pointer while calling {}",
                    stringify!(#il2cpp_method_name)
                ));
                return Err(::il2cpp_runtime::errors::Il2CppError::NullPointerDereference);
            }
        }
    } else {
        quote! {}
    };

    let il2cpp_return_type: syn::Type = match &function_def.sig.output {
        syn::ReturnType::Default => syn::parse_quote!(()),
        syn::ReturnType::Type(_, ty) => (**ty).clone(),
    };

    let mut function_signature = function_def.sig.clone();
    function_signature.output = syn::parse_quote!(-> Result<#il2cpp_return_type, Il2CppError>);

    let resolve_from_class_body = quote! {
        ::il2cpp_runtime::__log_debug(format_args!(
            "[il2cpp_method] Resolving {}::{} (args: {}, static: {})",
            class.qualified_name(),
            stringify!(#il2cpp_method_name),
            stringify!(#(#il2cpp_method_args),*),
            #is_static_method
        ));
        let method_info = match #class_method_lookup {
            Ok(method_info) => {
                ::il2cpp_runtime::__log_debug(format_args!(
                    "[il2cpp_method] Resolved {}::{}",
                    class.qualified_name(),
                    stringify!(#il2cpp_method_name)
                ));
                method_info
            }
            Err(e) => {
                // Duyệt lên các Base Class (Parent) nếu không tìm thấy ở class hiện tại
                let mut iter: *const usize = std::ptr::null();
                let mut resolved_from_parent = None;
                let mut current = class;

                loop {
                    let parent = ::il2cpp_runtime::api::il2cpp_class_get_parent(current, &iter);
                    if parent.0.is_null() || parent.0 == current.0 {
                        break;
                    }

                    match #parent_method_lookup {
                        Ok(method_info) => {
                            ::il2cpp_runtime::__log_debug(format_args!(
                                "[il2cpp_method] Resolved {}::{} via parent {}",
                                class.qualified_name(),
                                stringify!(#il2cpp_method_name),
                                parent.qualified_name()
                            ));
                            resolved_from_parent = Some(method_info);
                            break;
                        }
                        Err(_) => {
                            current = parent;
                        }
                    }
                }

                match resolved_from_parent {
                    Some(method_info) => method_info,
                    None => {
                        log_method_error(format_args!(
                            "[il2cpp_method] Failed to resolve {}::{}: {}",
                            class.qualified_name(),
                            stringify!(#il2cpp_method_name),
                            e
                        ));
                        return Err(e);
                    }
                }
            }
        };

        let va = method_info.va();
        if va.is_null() {
            let err = ::il2cpp_runtime::errors::Il2CppError::NullMethodPointer(
                format!("{}::{}", class.qualified_name(), #il2cpp_method_name)
            );
            log_method_error(format_args!("[il2cpp_method] {}", err));
            return Err(err);
        }

        Ok(unsafe { std::mem::transmute::<*const std::ffi::c_void, FuncType>(va) })
    };

    let method_binding = if is_static_method || macro_args.extension || macro_args.value_type {
        // Static / Extension / ValueType: Class cố định, cache trực tiếp con trỏ hàm
        quote! {
            type FuncType = extern "C" fn(#extern_fn_params) -> #il2cpp_return_type;

            static IL2CPP_METHOD_CACHE: std::sync::OnceLock<
                Result<FuncType, ::il2cpp_runtime::errors::Il2CppError>
            > = std::sync::OnceLock::new();

            let method: FuncType = match IL2CPP_METHOD_CACHE.get_or_init(|| {
                let class = match Self::get_class_static() {
                    Ok(class) => class,
                    Err(e) => {
                        log_method_error(format_args!(
                            "[il2cpp_method] Failed to get static class for {}: {}",
                            stringify!(#il2cpp_method_name),
                            e
                        ));
                        return Err(e);
                    }
                };
                #resolve_from_class_body
            }) {
                Ok(f) => *f,
                Err(e) => return Err(e.clone()),
            };
        }
    } else {
        // Instance Method trên Reference Type:
        // Cache cặp (Il2CppClass, FuncType) của lần gọi thành công đầu tiên.
        // - Nếu các lần gọi sau có cùng Il2CppClass (99.9% trường hợp): dùng ngay lập tức (O(1)).
        // - Nếu gặp object đa hình khác Il2CppClass: tự động resolve chuẩn cho class đó mà không crash!
        // - Nếu lần đầu lỗi: không khóa chết lỗi vào OnceLock mà cho phép thử lại.
        quote! {
            type FuncType = extern "C" fn(#extern_fn_params) -> #il2cpp_return_type;

            let resolve_for_class = |class: ::il2cpp_runtime::Il2CppClass| -> Result<FuncType, ::il2cpp_runtime::errors::Il2CppError> {
                #resolve_from_class_body
            };

            let runtime_class = self.get_class();
            if runtime_class.0.is_null() {
                return Err(::il2cpp_runtime::errors::Il2CppError::NullPointerDereference);
            }

            static INSTANCE_METHOD_CACHE: std::sync::OnceLock<(::il2cpp_runtime::Il2CppClass, FuncType)> =
                std::sync::OnceLock::new();

            let method: FuncType = if let Some((cached_class, cached_fn)) = INSTANCE_METHOD_CACHE.get() {
                if *cached_class == runtime_class {
                    *cached_fn
                } else {
                    // Cảnh báo khi phát hiện hàm được gọi đa hình (Virtual / Interface / Base class)
                    ::il2cpp_runtime::__log_debug(format_args!(
                        "[il2cpp_method] [POLYMORPHIC] {}::{} switched runtime class! Cached: '{}' -> Actual: '{}'",
                        Self::ffi_name(),
                        #il2cpp_method_name,
                        cached_class.qualified_name(),
                        runtime_class.qualified_name()
                    ));
                    resolve_for_class(runtime_class)?
                }
            } else {
                let actual_name = runtime_class.qualified_name();
                if actual_name != Self::ffi_name() {
                    // Log khi phát hiện struct khai báo ở Rust là Base/Interface nhưng object thực tế là Subclass
                    ::il2cpp_runtime::__log_debug(format_args!(
                        "[il2cpp_method] [INHERITANCE/INTERFACE] Declared '{}' but runtime instance is '{}' when resolving '{}'",
                        Self::ffi_name(),
                        actual_name,
                        #il2cpp_method_name
                    ));
                }
                let resolved_fn = resolve_for_class(runtime_class)?;
                ::il2cpp_runtime::__log_debug(format_args!(
                    "[il2cpp_method] [CACHE INIT] Cached {}::{} for runtime class '{}' at VA {:p}",
                    Self::ffi_name(),
                    #il2cpp_method_name,
                    actual_name,
                    resolved_fn as *const ()
                ));
                let _ = INSTANCE_METHOD_CACHE.set((runtime_class, resolved_fn));
                resolved_fn
            };
        }
    };

    let expanded = quote! {
        #function_vis unsafe #function_signature {
            fn log_method_error(args: std::fmt::Arguments<'_>) {
                ::il2cpp_runtime::__log_error(args);
            }

            #runtime_null_check
            #method_binding

            ::il2cpp_runtime::__log_debug(format_args!(
                "[il2cpp_method] Calling {}",
                stringify!(#il2cpp_method_name)
            ));
            Ok(#method_call)
        }
    };

    TokenStream::from(expanded)
}

/// Extract parameter names and types from function signature, skipping `self`
fn extract_function_parameters(function_def: &ItemFn) -> (Vec<syn::Ident>, Vec<syn::Type>) {
    let mut parameter_names: Vec<syn::Ident> = Vec::new();
    let mut parameter_types: Vec<syn::Type> = Vec::new();

    for arg in function_def.sig.inputs.iter() {
        match arg {
            FnArg::Receiver(_) => {}
            FnArg::Typed(pat_type) => {
                if let Pat::Ident(pat_ident) = &*pat_type.pat {
                    let param_name = pat_ident.ident.clone();
                    let param_type = *pat_type.ty.clone();

                    parameter_names.push(param_name);
                    parameter_types.push(param_type);
                }
            }
        }
    }
    (parameter_names, parameter_types)
}

fn impl_il2cpp_object(
    ident: &Ident,
    ffi_name: &LitStr,
    as_ptr_expr: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    quote! {
        impl Il2CppObject for #ident {
            fn ffi_name() -> &'static str {
                #ffi_name
            }
            fn as_ptr(&self) -> *const std::ffi::c_void {
                #as_ptr_expr
            }
        }
    }
}

fn impl_il2cpp_ref_type(
    ident: &Ident,
    ffi_name: &LitStr,
    as_ptr_expr: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let object_impl = impl_il2cpp_object(ident, ffi_name, as_ptr_expr);
    quote! {
        impl Il2CppRefType for #ident {}
        #object_impl
    }
}

struct Il2CppTypeArgs {
    name: LitStr,
    base: Option<syn::Type>,
}

enum Il2CppTypeArgItem {
    Name(LitStr),
    Base(syn::Type),
}

struct Il2CppTypeArgList {
    items: Vec<Il2CppTypeArgItem>,
}

impl Parse for Il2CppTypeArgList {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut items = Vec::new();

        while !input.is_empty() {
            if input.peek(LitStr) {
                let lit: LitStr = input.parse()?;
                items.push(Il2CppTypeArgItem::Name(lit));
            } else if input.peek(Ident) && input.peek2(Token![=]) {
                let ident: Ident = input.parse()?;
                let _eq: Token![=] = input.parse()?;
                if ident == "name" {
                    let lit: LitStr = input.parse()?;
                    items.push(Il2CppTypeArgItem::Name(lit));
                } else {
                    return Err(syn::Error::new_spanned(
                        ident,
                        "Unsupported argument; use name = \"Type\"",
                    ));
                }
            } else if input.peek(Ident) && input.peek2(Paren) {
                let ident: Ident = input.parse()?;
                if ident == "base" {
                    let content;
                    syn::parenthesized!(content in input);
                    let ty: syn::Type = content.parse()?;
                    items.push(Il2CppTypeArgItem::Base(ty));
                } else {
                    return Err(syn::Error::new_spanned(
                        ident,
                        "Unsupported argument; use base(Type)",
                    ));
                }
            } else {
                return Err(input.error("Unsupported argument; use a name string or base(Type)"));
            }

            if input.peek(Token![,]) {
                let _comma: Token![,] = input.parse()?;
            }
        }

        Ok(Self { items })
    }
}

fn parse_il2cpp_type_args(attr: TokenStream) -> Result<Il2CppTypeArgs, TokenStream> {
    if let Ok(name_only) = syn::parse::<LitStr>(attr.clone()) {
        return Ok(Il2CppTypeArgs {
            name: name_only,
            base: None,
        });
    }

    let mut name: Option<LitStr> = None;
    let mut base: Option<syn::Type> = None;

    let args = match syn::parse::<Il2CppTypeArgList>(attr) {
        Ok(args) => args,
        Err(err) => return Err(err.to_compile_error().into()),
    };

    for arg in args.items {
        match arg {
            Il2CppTypeArgItem::Name(lit) => {
                if name.is_some() {
                    return Err(
                        syn::Error::new_spanned(lit, "Multiple name arguments provided")
                            .to_compile_error()
                            .into(),
                    );
                }
                name = Some(lit);
            }
            Il2CppTypeArgItem::Base(ty) => {
                if base.is_some() {
                    return Err(
                        syn::Error::new_spanned(ty, "Multiple base arguments provided")
                            .to_compile_error()
                            .into(),
                    );
                }
                base = Some(ty);
            }
        }
    }

    let name = match name {
        Some(name) => name,
        None => {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "Missing required name string",
            )
            .to_compile_error()
            .into());
        }
    };

    Ok(Il2CppTypeArgs { name, base })
}

/// Generates a reference type wrapper for an IL2CPP class (managed reference type).
#[proc_macro_attribute]
pub fn il2cpp_ref_type(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = match parse_il2cpp_type_args(attr) {
        Ok(args) => args,
        Err(err) => return err,
    };

    let ItemStruct { ident, .. } = parse_macro_input!(item as ItemStruct);

    let ffi_type_expanded = generate_ffi_type_struct(&ident);
    let ref_impls = impl_il2cpp_ref_type(&ident, &args.name, quote! { self.0 });
    let base_helpers = if let Some(base_ty) = args.base {
        quote! {
            impl #ident {
                pub fn as_base(&self) -> #base_ty {
                    #base_ty(self.0)
                }
            }

            impl From<&#ident> for #base_ty {
                fn from(value: &#ident) -> Self {
                    #base_ty(value.0)
                }
            }
        }
    } else {
        quote! {}
    };
    let expanded = quote! {
        #ffi_type_expanded
        #ref_impls
        #base_helpers
    };

    TokenStream::from(expanded)
}

/// Generates a value type wrapper plus a `__Boxed` ref wrapper for IL2CPP value types.
/// The original struct is emitted as `#[repr(C)]` and `Copy`.
#[proc_macro_attribute]
pub fn il2cpp_value_type(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = match parse_il2cpp_type_args(attr) {
        Ok(args) => args,
        Err(err) => return err,
    };
    let input = parse_macro_input!(item as ItemStruct);
    let ident = &input.ident;
    let boxed_ident = Ident::new(&format!("{}__Boxed", ident), ident.span());

    let value_object_impl = impl_il2cpp_object(
        ident,
        &args.name,
        quote! { self as *const _ as *const std::ffi::c_void },
    );
    let boxed_ref_impls = impl_il2cpp_ref_type(&boxed_ident, &args.name, quote! { self.0 });
    let base_helpers = if let Some(base_ty) = args.base {
        quote! {
            impl #boxed_ident {
                pub fn as_base(&self) -> #base_ty {
                    #base_ty(self.0)
                }
            }

            impl From<&#boxed_ident> for #base_ty {
                fn from(value: &#boxed_ident) -> Self {
                    #base_ty(value.0)
                }
            }
        }
    } else {
        quote! {}
    };

    let expanded = quote! {
        #[repr(C)]
        #[derive(Debug, Copy, Clone)]
        #input

        #[repr(transparent)]
        #[derive(Debug, Copy, Clone, Eq, PartialEq)]
        pub struct #boxed_ident(pub *const std::ffi::c_void);

        unsafe impl Send for #boxed_ident {}
        unsafe impl Sync for #boxed_ident {}

        impl std::ops::Deref for #boxed_ident {
            type Target = #ident;

            fn deref(&self) -> &Self::Target {
                if self.0.is_null() {
                    panic!("Dereferenced null IL2CPP boxed value");
                }
                unsafe { &*(self.0 as *const #ident).byte_offset(0x10) }
            }
        }

        impl #boxed_ident {
            pub fn try_deref(&self) -> Result<&#ident, Il2CppError> {
                if self.0.is_null() {
                    Err(Il2CppError::NullPointerDereference)
                } else {
                    Ok(unsafe { &*(self.0 as *const #ident).byte_offset(0x10) })
                }
            }

            pub unsafe fn unbox(&self) -> Result<#ident, Il2CppError> {
                if self.0.is_null() {
                    Err(Il2CppError::NullPointerDereference)
                } else {
                    Ok(unsafe { *(self.0 as *const #ident).byte_offset(0x10) })
                }
            }
        }
        #value_object_impl

        impl Il2CppValueType for #ident {}

        #boxed_ref_impls
        #base_helpers
    };

    TokenStream::from(expanded)
}

/// Generates a raw FFI pointer wrapper type for IL2CPP internal structs.
#[proc_macro_attribute]
pub fn ffi_type(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let ItemStruct { ident, .. } = parse_macro_input!(item as ItemStruct);
    let generated = generate_ffi_type_struct(&ident);

    TokenStream::from(generated)
}

/// Generate the FFI transparent struct wrapper
fn generate_ffi_type_struct(struct_ident: &syn::Ident) -> proc_macro2::TokenStream {
    quote! {
        #[repr(transparent)]
        #[derive(Debug, Copy, Clone, Eq, PartialEq)]
        pub struct #struct_ident(pub *const std::ffi::c_void);


        unsafe impl Send for #struct_ident {}
        unsafe impl Sync for #struct_ident {}
    }
}

#[derive(Debug, FromMeta)]
#[darling(derive_syn_parse)]
struct FieldMacroArgs {
    name: LitStr,
}

/// Generates a field getter that returns `Result<T, Il2CppError>`.
/// **Do not return ValueType structs** from `#[il2cpp_field]`; return the boxed/ref
/// wrapper instead, because fields are accessed via object references.
#[proc_macro_attribute]
pub fn il2cpp_field(args: TokenStream, input: TokenStream) -> TokenStream {
    let field_args: FieldMacroArgs = match syn::parse(args) {
        Ok(parsed_args) => parsed_args,
        Err(parse_error) => {
            return parse_error.to_compile_error().into();
        }
    };

    let function_def: ItemFn = syn::parse_macro_input!(input as ItemFn);
    let rust_field_name = &function_def.sig.ident;
    let il2cpp_field_name = &field_args.name;

    let field_return_type = match &function_def.sig.output {
        syn::ReturnType::Default => {
            return syn::Error::new_spanned(
                &function_def.sig,
                "Field function must have explicit return type",
            )
            .to_compile_error()
            .into();
        }
        syn::ReturnType::Type(_, ty) => ty,
    };

    let has_self = function_def
        .sig
        .inputs
        .iter()
        .any(|arg| matches!(arg, FnArg::Receiver(_)));

    let param_count = function_def.sig.inputs.len();
    let expected_param_count = if has_self { 1 } else { 0 };

    if param_count != expected_param_count {
        let error_message = format!(
            "[il2cpp_field] ERROR: Field function '{}' must have no parameters (only optional self), but found {} parameters",
            rust_field_name,
            if has_self {
                param_count - 1
            } else {
                param_count
            }
        );
        let compile_error =
            syn::Error::new_spanned(&function_def.sig.inputs, error_message).to_compile_error();

        return TokenStream::from(compile_error);
    }

    let receiver = if has_self {
        quote! { &self }
    } else {
        quote! {}
    };

    let null_check = if has_self {
        quote! {
            if self.0.is_null() {
                log_field_error(format_args!(
                    "[il2cpp_field] Null self pointer while resolving {}",
                    #il2cpp_field_name
                ));
                return Err(::il2cpp_runtime::errors::Il2CppError::NullPointerDereference);
            }
        }
    } else {
        quote! {}
    };

    let class_expr = if has_self {
        quote! { self.get_class() }
    } else {
        quote! { Self::get_class_static()? }
    };

    let instance_expr = if has_self {
        quote! { self.0 }
    } else {
        quote! { core::ptr::null() }
    };

    let expanded = quote! {
        pub fn #rust_field_name(#receiver) -> Result<#field_return_type, Il2CppError>
        where
            #field_return_type: ::il2cpp_runtime::Il2CppRefType,
        {
            fn log_field_error(args: std::fmt::Arguments<'_>) {
                ::il2cpp_runtime::__log_error(args);
            }

            #null_check

            // Cache (Il2CppClass, Il2CppField) an toàn với cả tính đa hình
            static FIELD_CACHE: std::sync::OnceLock<(
                ::il2cpp_runtime::Il2CppClass,
                ::il2cpp_runtime::Il2CppField,
            )> = std::sync::OnceLock::new();

            let class = #class_expr;
            let field_info = if let Some((cached_class, cached_field)) = FIELD_CACHE.get() {
                if *cached_class == class {
                    *cached_field
                } else {
                    ::il2cpp_runtime::__log_debug(format_args!(
                        "[il2cpp_field] [POLYMORPHIC] {}::{} switched class '{}' -> '{}'",
                        Self::ffi_name(),
                        #il2cpp_field_name,
                        cached_class.qualified_name(),
                        class.qualified_name()
                    ));
                    class.find_field(#il2cpp_field_name)?
                }
            } else {
                ::il2cpp_runtime::__log_debug(format_args!(
                    "[il2cpp_field] Resolving {}::{}",
                    class.qualified_name(),
                    #il2cpp_field_name
                ));
                match class.find_field(#il2cpp_field_name) {
                    Ok(f) => {
                        let _ = FIELD_CACHE.set((class, f));
                        f
                    }
                    Err(e) => {
                        log_field_error(format_args!(
                            "[il2cpp_field] Failed to resolve {}::{}: {}",
                            class.qualified_name(),
                            #il2cpp_field_name,
                            e
                        ));
                        return Err(e);
                    }
                }
            };

            let value = match field_info.get_value(#instance_expr) {
                Ok(value) => value,
                Err(e) => {
                    log_field_error(format_args!(
                        "[il2cpp_field] Failed to read {}: {}",
                        #il2cpp_field_name,
                        e
                    ));
                    return Err(e);
                }
            };

            if value.is_null() {
                log_field_error(format_args!(
                    "[il2cpp_field] Null value for {}",
                    #il2cpp_field_name
                ));
                return Err(::il2cpp_runtime::errors::Il2CppError::NullPointerDereference);
            }

            Ok(unsafe { std::mem::transmute(value) })
        }
    };
    TokenStream::from(expanded)
}

#[derive(Debug, FromMeta)]
#[darling(derive_syn_parse)]
struct GetterPropertyArgs {
    property: LitStr,
}

/// Generates a property getter by expanding to `#[il2cpp_method(name = "get_<Property>", args = [])]`.
#[proc_macro_attribute]
pub fn il2cpp_getter_property(args: TokenStream, input: TokenStream) -> TokenStream {
    let getter_args: GetterPropertyArgs = match syn::parse(args) {
        Ok(parsed_args) => parsed_args,
        Err(parse_error) => return parse_error.to_compile_error().into(),
    };

    let function_def: ItemFn = parse_macro_input!(input as ItemFn);
    let getter_name = format!("get_{}", getter_args.property.value());

    let expanded = quote! {
        #[il2cpp_method(name = #getter_name, args = [])]
        #function_def
    };

    TokenStream::from(expanded)
}

/// Generates a repr'd enum with standard derives,
/// plus a `__Boxed` ref wrapper for IL2CPP value semantics.
#[proc_macro_attribute]
pub fn il2cpp_enum_type(attr: TokenStream, item: TokenStream) -> TokenStream {
    let repr_type = parse_macro_input!(attr as syn::Type);

    let mut enum_def = parse_macro_input!(item as ItemEnum);
    let ident = enum_def.ident.clone();
    let boxed_ident = Ident::new(&format!("{}__Boxed", ident), ident.span());
    let name_lit = LitStr::new(&ident.to_string(), ident.span());

    enum_def.attrs.push(syn::parse_quote!(#[repr(#repr_type)]));
    enum_def
        .attrs
        .push(syn::parse_quote!(#[derive(Debug, Copy, Clone, Eq, PartialEq, strum::EnumString, strum::Display)]));

    let enum_object_impl = impl_il2cpp_object(
        &ident,
        &name_lit,
        quote! { self as *const _ as *const std::ffi::c_void },
    );
    let boxed_ref_impls = impl_il2cpp_ref_type(&boxed_ident, &name_lit, quote! { self.0 });

    let boxed_struct = quote! {
        #[repr(transparent)]
        #[derive(Debug, Copy, Clone, Eq, PartialEq)]
        pub struct #boxed_ident(pub *const std::ffi::c_void);

        unsafe impl Send for #boxed_ident {}
        unsafe impl Sync for #boxed_ident {}

        impl std::ops::Deref for #boxed_ident {
            type Target = #ident;

            fn deref(&self) -> &Self::Target {
                if self.0.is_null() {
                    panic!("Dereferenced null IL2CPP boxed value");
                }
                unsafe { &*(self.0 as *const #ident).byte_offset(0x10) }
            }
        }

        impl #boxed_ident {
            pub fn try_deref(&self) -> Result<&#ident, Il2CppError> {
                if self.0.is_null() {
                    Err(Il2CppError::NullPointerDereference)
                } else {
                    Ok(unsafe { &*(self.0 as *const #ident).byte_offset(0x10) })
                }
            }

            pub unsafe fn unbox(&self) -> Result<#ident, Il2CppError> {
                if self.0.is_null() {
                    Err(Il2CppError::NullPointerDereference)
                } else {
                    Ok(unsafe { *(self.0 as *const #ident).byte_offset(0x10) })
                }
            }
        }
    };

    let expanded = quote! {
        #enum_def

        #boxed_struct

        #enum_object_impl

        impl Il2CppValueType for #ident {}

        #boxed_ref_impls
    };

    TokenStream::from(expanded)
}