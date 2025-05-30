// Copyright 2025-2030 PyFrame Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, parse_quote, parse_str,
    punctuated::Punctuated,
    token::{Comma, Semi},
    FnArg, ItemFn, LitInt, Pat, Stmt, Type,
};

fn is_option_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "Option";
        }
    }
    false
}

fn pyframe_api_args(api_inputs: Punctuated<FnArg, Comma>) -> Option<Stmt> {
    let len = api_inputs.len();

    if len == 0 {
        None
    } else {
        let mut names: Punctuated<Box<Pat>, Comma> = Punctuated::new();
        let mut types: Punctuated<Box<Type>, Comma> = Punctuated::new();
        let len = parse_str::<LitInt>(&len.to_string()).unwrap();

        for arg in api_inputs {
            if let FnArg::Typed(typed) = arg {
                names.push(typed.pat.clone());
                types.push(typed.ty.clone());
            }
        }

        let has_option = types.iter().any(|ty| is_option_type(ty));

        if has_option {
            Some(parse_quote! {
                let (#names,) = request.args().optional::<(#types,)>(#len)?;
            })
        } else {
            Some(parse_quote! {
                let (#names,) = request.args().get::<(#types,)>()?;
            })
        }
    }
}

#[proc_macro_attribute]
pub fn pyframe_api(_: TokenStream, raw_item: TokenStream) -> TokenStream {
    let define = parse_macro_input!(raw_item as ItemFn);

    let name = define.sig.ident;
    let inputs = define.sig.inputs;
    let output = define.sig.output;

    let mut stmts = Punctuated::<syn::Stmt, Semi>::new();
    define.block.stmts.into_iter().for_each(|stmt| {
        stmts.push(stmt);
    });

    let app_ty = quote! { std::sync::Arc<crate::CoreApplication> };
    let window_ty = quote! { std::sync::Arc<crate::window_manager::window::FrameWindow> };
    let request_ty = quote! { crate::api_manager::ApiRequest };

    let args = pyframe_api_args(inputs);

    TokenStream::from(quote! {
        fn #name(app: #app_ty, window: #window_ty, request: #request_ty) #output {
            #args
            #stmts
        }
    })
}

#[proc_macro_attribute]
pub fn pyframe_event_api(_: TokenStream, raw_item: TokenStream) -> TokenStream {
    let define = parse_macro_input!(raw_item as ItemFn);

    let name = define.sig.ident;
    let inputs = define.sig.inputs;
    let output = define.sig.output;

    let mut stmts = Punctuated::<syn::Stmt, Semi>::new();
    define.block.stmts.into_iter().for_each(|stmt| {
        stmts.push(stmt);
    });

    let app_ty = quote! { std::sync::Arc<crate::CoreApplication> };
    let window_ty = quote! { std::sync::Arc<crate::window_manager::window::FrameWindow> };
    let request_ty = quote! { crate::api_manager::ApiRequest };
    let target_ty = quote! { &crate::utils::FrameWindowTarget };
    let control_flow_ty = quote! { &mut tao::event_loop::ControlFlow };

    let args = pyframe_api_args(inputs);

    TokenStream::from(quote! {
        fn #name(app: #app_ty, window: #window_ty, request: #request_ty, target: #target_ty, control_flow: #control_flow_ty) #output {
            #args
            #stmts
        }
    })
}
