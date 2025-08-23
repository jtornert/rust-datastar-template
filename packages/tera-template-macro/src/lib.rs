use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::quote;
use syn::{parse_macro_input, Attribute, DeriveInput, Expr, ExprLit, Lit, LitStr, Meta};

struct Struct<'a> {
    name: &'a Ident,
    path: LitStr,
}

#[inline]
#[must_use]
fn is_template_attr(attr: &&Attribute) -> bool {
    attr.path().is_ident("template")
}

fn parse_args(input: &Attribute) -> syn::Result<Meta> {
    input.parse_args().map_err(|_| {
        syn::Error::new_spanned(
            input,
            "Expected attribute argument `path` in parentheses: \
        `#[template(path = \"index.html\")]`",
        )
    })
}

fn get_attr(input: &DeriveInput) -> syn::Result<&Attribute> {
    input.attrs.iter().find(is_template_attr).ok_or_else(|| {
        syn::Error::new_spanned(input, "The #[template(path = \"...\")] is required.")
    })
}

fn get_path_name_value(meta: Meta) -> syn::Result<ExprLit> {
    let Meta::NameValue(nv) = meta else {
        return Err(syn::Error::new_spanned(
            meta,
            "Expected `path = \"...\"`. E.g. `#[template(path = \"index.html\")]`",
        ));
    };

    if !nv.path.is_ident("path") {
        return Err(syn::Error::new_spanned(&nv.path, "Expected `path`."));
    }

    match nv.value {
        Expr::Lit(lit) => Ok(lit),
        _ => Err(syn::Error::new_spanned(
            nv,
            "The assignment to `path` must be a string literal.",
        )),
    }
}

fn get_lit_str(lit: ExprLit) -> syn::Result<LitStr> {
    match lit.lit {
        Lit::Str(res) => Ok(res),
        _ => Err(syn::Error::new_spanned(
            lit,
            "The assignment to `path` must be a string literal.",
        )),
    }
}

fn get_path_value(input: &Attribute) -> syn::Result<LitStr> {
    parse_args(input)
        .and_then(get_path_name_value)
        .and_then(get_lit_str)
}

impl<'a> Struct<'a> {
    fn from_syn(input: &'a DeriveInput) -> syn::Result<Self> {
        let name = &input.ident;
        get_attr(input)
            .and_then(get_path_value)
            .map(move |path| Self { name, path })
    }
}

fn expand_struct(s: Struct) -> proc_macro2::TokenStream {
    let Struct { name, path } = s;

    let filepath = std::path::Path::new(
        &std::env::var("CARGO_MANIFEST_DIR").expect("could not read project root directory"),
    )
    .join("templates")
    .join(&path.value());

    if !filepath.exists() {
        return syn::Error::new(
            path.span(),
            format!(
                "Template path {:?} does not exist",
                &filepath.to_string_lossy()
            ),
        )
        .into_compile_error()
        .into();
    }

    quote! {
        #[automatically_derived]
        impl #name {
            pub fn render(&self, tera: impl std::ops::Deref<Target = tera::Tera>, locale: &str) -> String {
                let mut context = tera::Context::from_serialize(self).expect("Failed to create context");
                context.insert("locale", locale);
                let rendered =  tera
                    .render(#path, &context)
                    .expect("Failed to render template");
                rendered
            }
        }
    }
}

/// Create a template from some struct
///
/// # Attribute
///
/// This macro requires the #[template(path = "...")] to be provided, this tells the derived
/// template where to look for the file it renders.
///
/// ## Named Arguments
///
/// * path - A string literal ("hello world"), which contains the path to the file which will
///   be rendered. E.g. path = "index.html".
///
/// # Example
///
///
/// ```rust
/// use tera_hot_reload::TeraTemplate;
///
/// #[derive(serde::Serialize, TeraTemplate)]
/// #[template(path = "index.html")]
/// struct MyTemplate {
///     hello: String,
///     world: String
/// }
/// ```
#[proc_macro_derive(TeraTemplate, attributes(template))]
pub fn tera_template_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    Struct::from_syn(&input)
        .map_or_else(syn::Error::into_compile_error, expand_struct)
        .into()
}
