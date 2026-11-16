//! macOS Quick Look previews of SolidCraft designs: the saved picture, features and parameters.
//!
//! A data-based Quick Look preview extension (`SolidCraft.app/Contents/PlugIns`, macOS 12 and
//! later). Pressing Space on a design in Finder shows a card (`solidcraft_design_preview::card`):
//! the picture a desktop save keeps in the design, the timeline's feature names and the user's
//! parameters. Everything comes from the design's own `cache/thumbnail.png` and `document.json`,
//! read with `solidcraft-design-preview`; nothing evaluates, meshes or draws the model.

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "macos")]
fn main() {
    std::process::exit(macos::run(provider::register));
}

/// The extension exists only on macOS; elsewhere the binary does nothing.
#[cfg(not(target_os = "macos"))]
fn main() {}

/// The extension's principal class (`NSExtensionPrincipalClass` in its Info.plist).
#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
mod provider {
    use std::ptr::{self, NonNull};

    use block2::{DynBlock, RcBlock};
    use objc2::rc::Retained;
    use objc2::{AnyThread, ClassType, define_class};
    use objc2_core_foundation::CGSize;
    use objc2_foundation::{NSData, NSDictionary, NSError, NSObjectProtocol, NSString};
    use objc2_quick_look_ui::{QLFilePreviewRequest, QLPreviewProvider, QLPreviewReply, QLPreviewReplyAttachment, QLPreviewingController};
    use objc2_uniform_type_identifiers::{UTTypeHTML, UTTypePNG};

    use crate::macos;

    /// The card's picture, as the page refers to it (`cid:` + the attachment's key).
    const PICTURE: &str = "picture";
    /// The card's size before it is laid out, in points: room for a picture and two short lists.
    const CARD: (f64, f64) = (800.0, 600.0);

    define_class!(
        // SAFETY: QLPreviewProvider's subclasses implement QLPreviewingController's data-based
        // method, as this one does; the class adds no ivars or Drop.
        #[unsafe(super(QLPreviewProvider))]
        #[name = "SolidCraftPreviewProvider"]
        pub struct PreviewProvider;

        unsafe impl NSObjectProtocol for PreviewProvider {}

        // SAFETY: the method's signature is the protocol's for this selector.
        unsafe impl QLPreviewingController for PreviewProvider {
            #[unsafe(method(providePreviewForFileRequest:completionHandler:))]
            fn provide(&self, request: &QLFilePreviewRequest, handler: &DynBlock<dyn Fn(*mut QLPreviewReply, *mut NSError)>) {
                match reply(request) {
                    Some(reply) => handler.call((Retained::as_ptr(&reply).cast_mut(), ptr::null_mut())),
                    None => {
                        let error = macos::nothing_to_show();
                        handler.call((ptr::null_mut(), Retained::as_ptr(&error).cast_mut()));
                    }
                }
            }
        }
    );

    /// The card for the design `request` names; `None` when it isn't a design this can read.
    fn reply(request: &QLFilePreviewRequest) -> Option<Retained<QLPreviewReply>> {
        // SAFETY: a plain property read on the request the host handed us.
        let url = unsafe { request.fileURL() };
        let path = macos::file(&url)?;
        let summary = solidcraft_design_preview::summary(&path)?;
        let picture = solidcraft_design_preview::picture(&path);
        let src = format!("cid:{PICTURE}");
        let page = NSData::from_vec(solidcraft_design_preview::card(&summary, picture.as_ref().map(|_| src.as_str())).into_bytes());
        // The page is ready; the block only hands it over. Blocks return objects autoreleased.
        let data = RcBlock::new(move |_: NonNull<QLPreviewReply>, _: *mut *mut NSError| Retained::autorelease_return(page.clone()));
        // SAFETY: the block matches the data-creation block's signature and returns a valid
        // NSData; the reply keeps the block.
        let reply = unsafe {
            QLPreviewReply::initWithDataOfContentType_contentSize_dataCreationBlock(
                QLPreviewReply::alloc(),
                UTTypeHTML,
                CGSize::new(CARD.0, CARD.1),
                &data,
            )
        };
        if let Some(picture) = picture {
            // SAFETY: the attachment takes the picture's bytes and type; the reply copies the
            // dictionary.
            unsafe {
                let attachment =
                    QLPreviewReplyAttachment::initWithData_contentType(QLPreviewReplyAttachment::alloc(), &NSData::from_vec(picture.png), UTTypePNG);
                reply.setAttachments(&NSDictionary::from_slices(&[&*NSString::from_str(PICTURE)], &[&*attachment]));
            }
        }
        if !summary.name.is_empty() {
            // SAFETY: the reply copies the title.
            unsafe { reply.setTitle(&NSString::from_str(&summary.name)) };
        }
        Some(reply)
    }

    /// Register the class before the extension host looks it up by name.
    pub fn register() {
        PreviewProvider::class();
    }
}
