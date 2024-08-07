//! On the server, we collect any elements that should be rendered into the head in the first frame of SSR.
//! After the first frame, we have already sent down the head, so we can't modify it in place. The web client
//! will hydrate the head with the correct contents once it loads.

use std::cell::RefCell;

use dioxus_lib::{html::document::*, prelude::*};
use dioxus_ssr::Renderer;
use generational_box::GenerationalBox;

#[derive(Default)]
struct ServerDocumentInner {
    streaming: bool,
    title: Option<String>,
    meta: Vec<Element>,
    link: Vec<Element>,
    script: Vec<Element>,
}

/// A Document provider that collects all contents injected into the head for SSR rendering.
#[derive(Default)]
pub(crate) struct ServerDocument(RefCell<ServerDocumentInner>);

impl ServerDocument {
    pub(crate) fn render(
        &self,
        to: &mut impl std::fmt::Write,
        renderer: &mut Renderer,
    ) -> std::fmt::Result {
        fn lazy_app(props: Element) -> Element {
            props
        }
        let myself = self.0.borrow();
        let element = rsx! {
            if let Some(title) = myself.title.as_ref() {
                title { title: "{title}" }
            }
            {myself.meta.iter().map(|m| rsx! { {m} })}
            {myself.link.iter().map(|l| rsx! { {l} })}
            {myself.script.iter().map(|s| rsx! { {s} })}
        };

        let mut dom = VirtualDom::new_with_props(lazy_app, element);
        dom.rebuild_in_place();

        // We don't hydrate the head, so we can set the pre_render flag to false to save a few bytes
        let was_pre_rendering = renderer.pre_render;
        renderer.pre_render = false;
        renderer.render_to(to, &dom)?;
        renderer.pre_render = was_pre_rendering;

        Ok(())
    }

    pub(crate) fn start_streaming(&self) {
        self.0.borrow_mut().streaming = true;
    }

    pub(crate) fn warn_if_streaming(&self) {
        if self.0.borrow().streaming {
            tracing::warn!("Attempted to insert content into the head after the initial streaming frame. Inserting content into the head only works during the initial render of SSR outside before resolving any suspense boundaries.");
        }
    }

    /// Write the head element into the serialized context for hydration
    /// We write true if the head element was written to the DOM during server side rendering
    pub(crate) fn serialize_for_hydration(&self) {
        let serialize = crate::html_storage::serialize_context();
        serialize.push(&!self.0.borrow().streaming);
    }
}

impl Document for ServerDocument {
    fn new_evaluator(&self, js: String) -> GenerationalBox<Box<dyn Evaluator>> {
        NoOpDocument.new_evaluator(js)
    }

    fn set_title(&self, title: String) {
        self.warn_if_streaming();
        self.serialize_for_hydration();
        self.0.borrow_mut().title = Some(title);
    }

    fn create_meta(&self, props: MetaProps) {
        self.warn_if_streaming();
        self.serialize_for_hydration();
        self.0.borrow_mut().meta.push(rsx! {
            meta {
                name: props.name,
                charset: props.charset,
                http_equiv: props.http_equiv,
                content: props.content,
                property: props.property,
            }
        });
    }

    fn create_script(&self, props: ScriptProps) {
        self.warn_if_streaming();
        self.serialize_for_hydration();
        let children = props.script_contents();
        self.0.borrow_mut().script.push(rsx! {
            script {
                src: props.src,
                defer: props.defer,
                crossorigin: props.crossorigin,
                fetchpriority: props.fetchpriority,
                integrity: props.integrity,
                nomodule: props.nomodule,
                nonce: props.nonce,
                referrerpolicy: props.referrerpolicy,
                r#type: props.r#type,
                {children}
            }
        });
    }

    fn create_link(&self, props: head::LinkProps) {
        self.warn_if_streaming();
        self.serialize_for_hydration();
        self.0.borrow_mut().link.push(rsx! {
            link {
                rel: props.rel,
                media: props.media,
                title: props.title,
                disabled: props.disabled,
                r#as: props.r#as,
                sizes: props.sizes,
                href: props.href,
                crossorigin: props.crossorigin,
                referrerpolicy: props.referrerpolicy,
                fetchpriority: props.fetchpriority,
                hreflang: props.hreflang,
                integrity: props.integrity,
                r#type: props.r#type,
                blocking: props.blocking,
            }
        })
    }
}
