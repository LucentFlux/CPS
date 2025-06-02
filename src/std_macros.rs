pub mod concat;
pub mod include_str;
pub mod stringify;

macro_rules! impl_std_cps {
    (
        $(use $import:path;)*
        fn $name:ident($param_ident:ident : $param_ty:path $(,)?) -> $ret_ty:path {
            $( $impl_tt:tt )*
        }
    ) => {
        mod $name {
            #[allow(unused_imports)]
            use super::*;
            $(
                use $import;
            )*

            pub struct Impl {}

            impl crate::cps_proc_macro::CPSProcMacro for Impl {
                type Input = $param_ty;
                type Output = $ret_ty;

                fn step($param_ident: $param_ty) -> $ret_ty {
                    $( $impl_tt )*
                }
            }
        }

        pub fn $name(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
            let item = proc_macro2::TokenStream::from(item);
            let res = crate::cps_proc_macro::perform_macro::<$name::Impl>(item);
            proc_macro::TokenStream::from(res)
        }
    };
}

pub(crate) use impl_std_cps;
