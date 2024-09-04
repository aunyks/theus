use proc_macro::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::{parse_macro_input, Error, FnArg, ImplItem, ItemImpl, Pat, ReturnType, Type, Visibility};

const SCALAR_TYPES: &[&str] = &[
    "bool",  // Boolean type
    "char",  // Character type
    "i8",    // 8-bit signed integer
    "i16",   // 16-bit signed integer
    "i32",   // 32-bit signed integer
    "i64",   // 64-bit signed integer
    "i128",  // 128-bit signed integer
    "isize", // Pointer-sized signed integer
    "u8",    // 8-bit unsigned integer
    "u16",   // 16-bit unsigned integer
    "u32",   // 32-bit unsigned integer
    "u64",   // 64-bit unsigned integer
    "u128",  // 128-bit unsigned integer
    "usize", // Pointer-sized unsigned integer
    "f32",   // 32-bit floating point
    "f64",   // 64-bit floating point
];

fn modify_type_to_pointer(ty: &Type) -> proc_macro2::TokenStream {
    match ty {
        Type::Reference(type_reference) => {
            let elem = &type_reference.elem;
            if let Type::Slice(type_slice) = &**elem {
                let inner = &type_slice.elem;
                if type_reference.mutability.is_some() {
                    quote! { *mut #inner }
                } else {
                    quote! { *const #inner }
                }
            } else if type_reference.mutability.is_some() {
                quote! { *mut #elem }
            } else {
                quote! { *const #elem }
            }
        }
        Type::Ptr(ptr) => quote! { #ptr },
        Type::Path(type_path) => {
            if let Some(segment) = type_path.path.segments.last() {
                if SCALAR_TYPES.contains(&segment.ident.to_string().as_str()) {
                    return quote! { #ty };
                }
            }
            quote! { *mut #ty }
        }
        _ => quote! { *mut #ty },
    }
}

#[proc_macro_attribute]
pub fn c_compatible(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemImpl);
    let struct_name = &input.self_ty;

    let mut has_create = false;
    let mut has_destroy = false;

    let trait_name = input
        .trait_
        .as_ref()
        .map(|(_, path, _)| path.segments.last().unwrap().ident.to_string());

    let mut c_compatible_fns = vec![];
    let mut errors = vec![];

    for item in &input.items {
        if let ImplItem::Fn(method) = item {
            if trait_name.is_none() && !matches!(method.vis, Visibility::Public(_)) {
                continue;
            }
            let fn_name = &method.sig.ident;

            let mut is_create = false;
            let mut is_destroy = false;

            if fn_name == "create" {
                is_create = true;
                has_create = true;
            } else if fn_name == "destroy" {
                is_destroy = true;
                has_destroy = true;
            }

            let c_fn_name = if let Some(trait_name) = &trait_name {
                format_ident!(
                    "{}_{}_{}",
                    struct_name.to_token_stream().to_string().to_lowercase(),
                    trait_name.to_lowercase(),
                    fn_name
                )
            } else {
                format_ident!(
                    "{}_{}",
                    struct_name.to_token_stream().to_string().to_lowercase(),
                    fn_name
                )
            };

            let mut params = vec![];
            let mut args = vec![];
            let return_type = &method.sig.output;

            let mut self_param = None;
            for (i, arg) in method.sig.inputs.iter().enumerate() {
                match arg {
                    FnArg::Receiver(receiver) => {
                        if i != 0 {
                            let error = Error::new_spanned(
                                receiver,
                                "Self parameter must be the first parameter",
                            );
                            errors.push(error);
                            continue;
                        }
                        self_param = Some(receiver);
                    }
                    FnArg::Typed(pat_type) => {
                        if let Pat::Ident(pat_ident) = &*pat_type.pat {
                            let ident = &pat_ident.ident;
                            let ty = &*pat_type.ty;
                            let modified_ty = modify_type_to_pointer(ty);
                            if let Type::Reference(type_reference) = ty {
                                if let Type::Slice(_) = &*type_reference.elem {
                                    let ptr_ident = format_ident!("{}_ptr", ident);
                                    let len_ident = format_ident!("{}_len", ident);
                                    params.push(
                                        quote! { #ptr_ident: #modified_ty, #len_ident: usize },
                                    );
                                    args.push(quote! { #ident });
                                    continue;
                                }
                            }
                            params.push(quote! { #ident: #modified_ty });
                            args.push(quote! { #ident });
                        }
                    }
                }
            }

            let (ptr_type, self_expr, is_consuming) = if let Some(receiver) = self_param {
                if is_create {
                    errors.push(Error::new_spanned(
                        receiver,
                        "Create function cannot receive self as argument",
                    ));
                }
                if receiver.reference.is_none() {
                    (
                        quote! { *mut },
                        quote! { unsafe { Box::from_raw(ptr) } },
                        true,
                    )
                } else if receiver.mutability.is_some() {
                    if is_destroy {
                        errors.push(Error::new_spanned(
                            receiver,
                            "Destroy function must receive owned self argument. &mut self found instead"
                        ));
                    }
                    (quote! { *mut }, quote! { unsafe { &mut *ptr } }, false)
                } else {
                    if is_destroy {
                        errors.push(Error::new_spanned(
                            self_param,
                            "Destroy function must receive owned self argument. &self found instead"
                        ));
                    }
                    (quote! { *const }, quote! { unsafe { &*ptr } }, false)
                }
            } else {
                if is_destroy {
                    errors.push(Error::new_spanned(
                        self_param,
                        "Destroy function must receive owned self argument. Found no receiver",
                    ));
                }
                (quote! {}, quote! {}, false)
            };

            let fn_call = if self_param.is_some() {
                let args_converted = args.iter().zip(method.sig.inputs.iter().skip(1)).map(|(arg, input)| {
                    if let FnArg::Typed(pat_type) = input {
                        let ty = &*pat_type.ty;
                        match ty {
                            Type::Reference(type_reference) => {
                                if let Type::Slice(_) = &*type_reference.elem {
                                    let arg_str = arg.to_string();
                                    let ptr_ident = format_ident!("{}_ptr", arg_str);
                                    let len_ident = format_ident!("{}_len", arg_str);
                                    if type_reference.mutability.is_some() {
                                        quote! { unsafe { core::slice::from_raw_parts_mut(#ptr_ident, #len_ident) } }
                                    } else {
                                        quote! { unsafe { core::slice::from_raw_parts(#ptr_ident, #len_ident) } }
                                    }
                                } else if type_reference.mutability.is_some() {
                                    quote! { unsafe { &mut *#arg } }
                                } else {
                                    quote! { unsafe { &*#arg } }
                                }
                            }
                            Type::Ptr(_) => quote! { #arg },
                            Type::Path(type_path) => {
                                if let Some(segment) = type_path.path.segments.last() {
                                    if type_path.path.leading_colon.is_none() && segment.arguments.is_empty() && !SCALAR_TYPES.contains(&segment.ident.to_string().as_str()) {
                                        errors.push(
                                            Error::new_spanned(
                                                type_path,
                                                "Only scalar types can be passed directly to functions without being behind refs or pointers"
                                            )
                                        );
                                    }
                                };
                                quote! { #arg }
                            }
                            _ => {
                                errors.push(
                                    Error::new_spanned(
                                        arg,
                                        "Only scalar types can be passed directly to functions without being behind refs or pointers"
                                    )
                                );
                                quote! { #arg }
                            },
                        }
                    } else {
                        quote! { #arg }
                    }
                });
                if is_consuming {
                    quote! { #self_expr.#fn_name(#(#args_converted),*) }
                } else {
                    quote! { (#self_expr).#fn_name(#(#args_converted),*) }
                }
            } else {
                let args_converted = args.iter().zip(method.sig.inputs.iter()).map(|(arg, input)| {
                    if let FnArg::Typed(pat_type) = input {
                        let ty = &*pat_type.ty;
                        match ty {
                            Type::Reference(type_reference) => {
                                if let Type::Slice(_) = &*type_reference.elem {
                                    let arg_str = arg.to_string();
                                    let ptr_ident = format_ident!("{}_ptr", arg_str);
                                    let len_ident = format_ident!("{}_len", arg_str);
                                    if type_reference.mutability.is_some() {
                                        quote! { unsafe { core::slice::from_raw_parts_mut(#ptr_ident, #len_ident) } }
                                    } else {
                                        quote! { unsafe { core::slice::from_raw_parts(#ptr_ident, #len_ident) } }
                                    }
                                } else if type_reference.mutability.is_some() {
                                    quote! { unsafe { &mut *#arg } }
                                } else {
                                    quote! { unsafe { &*#arg } }
                                }
                            }
                            Type::Path(type_path) => {
                                if let Some(segment) = type_path.path.segments.last() {
                                    if type_path.path.leading_colon.is_none() && segment.arguments.is_empty() && !SCALAR_TYPES.contains(&segment.ident.to_string().as_str()) {
                                        errors.push(
                                            Error::new_spanned(
                                                type_path,
                                                "Only scalar types can be passed directly to functions without being behind refs or pointers"
                                            )
                                        );
                                    }
                                };
                                quote! { #arg }
                            }
                            _ => {
                                errors.push(
                                    Error::new_spanned(
                                        arg,
                                        "Only scalar types can be passed directly to functions without being behind refs or pointers"
                                    )
                                );
                                quote! { #arg }
                            },
                        }
                    } else {
                        quote! { #arg }
                    }
                });
                quote! { #struct_name::#fn_name(#(#args_converted),*) }
            };

            // Handle Self return type
            let (modified_return_type, return_expr) = match return_type {
                ReturnType::Type(_, ty) => {
                    if let Type::Path(type_path) = &**ty {
                        if let Some(segment) = type_path.path.segments.last() {
                            if !SCALAR_TYPES.contains(&segment.ident.to_string().as_str()) {
                                let owned_return_type = &segment.ident;
                                if owned_return_type == "Self" {
                                    (
                                        quote! { -> *mut #struct_name },
                                        quote! { Box::into_raw(Box::new(#fn_call)) },
                                    )
                                } else {
                                    (
                                        quote! { -> *mut #type_path },
                                        quote! { Box::into_raw(Box::new(#fn_call)) },
                                    )
                                }
                            } else {
                                (quote! { #return_type }, quote! { #fn_call })
                            }
                        } else {
                            (quote! { #type_path }, quote! { #fn_call })
                        }
                    } else if let Type::Reference(refr) = &**ty {
                        let inner_type = &*refr.elem;
                        if refr.mutability.is_some() {
                            (
                                quote! { -> *mut #inner_type  },
                                quote! { #fn_call as *mut #inner_type },
                            )
                        } else {
                            (
                                quote! { -> *const #inner_type  },
                                quote! { #fn_call as *const #inner_type },
                            )
                        }
                    } else {
                        (quote! { #return_type }, quote! { #fn_call })
                    }
                }
                _ => (quote! { #return_type }, quote! { #fn_call }),
            };

            // Preserve doc comments
            let doc_comments = method
                .attrs
                .iter()
                .filter(|attr| attr.path().is_ident("doc"))
                .collect::<Vec<_>>();

            let wrapper_fn = if self_param.is_some() {
                quote! {
                    #(#doc_comments)*
                    #[no_mangle]
                    pub extern "C" fn #c_fn_name(ptr: #ptr_type #struct_name, #(#params),*) #modified_return_type {
                        // SAFETY: This function is unsafe because it works with raw pointers.
                        // The caller must ensure that all pointers are valid and properly aligned.
                        #return_expr
                    }
                }
            } else {
                quote! {
                    #(#doc_comments)*
                    #[no_mangle]
                    pub unsafe extern "C" fn #c_fn_name(#(#params),*) #modified_return_type {
                        // SAFETY: This function is unsafe because it works with raw pointers.
                        // The caller must ensure that all pointers are valid and properly aligned.
                        #return_expr
                    }
                }
            };

            c_compatible_fns.push(wrapper_fn);
        }
    }

    if trait_name.is_none() && (!has_create || !has_destroy) {
        let missing = if !has_create { "create" } else { "destroy" };
        errors.push(Error::new_spanned(
            &input,
            format!(
                "Struct must have both 'create' and 'destroy' functions. Missing: {}",
                missing
            ),
        ));
    }

    if !errors.is_empty() {
        let compile_errors = errors.iter().map(Error::to_compile_error);
        return quote! {
            #(#compile_errors)*
        }
        .into();
    }

    let expanded = quote! {
        #input

        #(#c_compatible_fns)*
    };

    expanded.into()
}
