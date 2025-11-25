use egui_async::{Bind, EguiAsyncPlugin};
use egui::load::Bytes;

const XOR_IMAGE_URI: &'static str = "bytes://xor.bmp";

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    width: u32,

    page_number: u16,
    #[serde(skip)]
    pdf_before: Bind<pdfium_render::prelude::PdfDocument<'static>, String>,
    #[serde(skip)]
    pdf_after: Bind<pdfium_render::prelude::PdfDocument<'static>, String>,

    #[serde(skip)]
    image_xor: Option<Bytes>,

    #[serde(skip)]
    pdfium: &'static pdfium_render::prelude::Pdfium,

    // #[serde(skip)]
    // xor_image: Option<egui::load::Bytes>,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            width: 2048,
            page_number: 0,
            pdf_before: Bind::default(),
            pdf_after: Bind::default(),
            image_xor: None,
            pdfium: Box::leak(Box::new(Default::default())),
        }
    }
}

impl TemplateApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            Default::default()
        }
    }
}

impl eframe::App for TemplateApp {
    /// Called by the framework to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // This registers the plugin that drives the async event loop.
        // It's idempotent and cheap to call on every frame.
        ctx.plugin_or_default::<EguiAsyncPlugin>();

        // Put your widgets into a `SidePanel`, `TopBottomPanel`, `CentralPanel`, `Window` or `Area`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        // egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
        //     // The top panel is often a good place for a menu bar:
        //     egui::MenuBar::new().ui(ui, |ui| {
        //         // NOTE: no File->Quit on web pages!
        //         let is_web = cfg!(target_arch = "wasm32");
        //         if !is_web {
        //             ui.menu_button("File", |ui| {
        //                 if ui.button("Quit").clicked() {
        //                     ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        //                 }
        //             });
        //             ui.add_space(16.0);
        //         }

        //         egui::widgets::global_theme_preference_buttons(ui);
        //     });
        // });

        egui::SidePanel::left("left_panel").show(ctx, |ui| {
            ui.heading("Drawing Diff");

            ui.separator();

            let mut changed = false;

            changed |= pdf_picker(
                ui,
                "Before…",
                &mut self.pdf_before,
                &self.pdfium,
            ).changed();
            changed |= pdf_picker(
                ui,
                "After…",
                &mut self.pdf_after,
                &self.pdfium,
            ).changed();

            if let Some(pdf_before) = self.pdf_before.ok_ref() && let Some(pdf_after) = self.pdf_after.ok_ref() {
                let page_count = pdf_before.pages().len().min(pdf_after.pages().len());
                let last_page = page_count.saturating_sub(1);
                self.page_number = self.page_number.clamp(0, last_page);
                if page_count > 1 {
                    changed |= ui.add(egui::Slider::new(&mut self.page_number, 0..=last_page).text("Page")).changed();
                }
            }

            if changed {
                self.image_xor = None;
                ctx.forget_image(XOR_IMAGE_URI);
            }

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                powered_by_egui_and_eframe(ui);
                egui::warn_if_debug_build(ui);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's
            if let Some(ref xor_image) = self.image_xor {
                ui.centered_and_justified(|ui| {
                    ui.add(egui::Image::from_bytes(
                        XOR_IMAGE_URI,
                        xor_image.clone(),
                    ));
                });
            } else if let Some(pdf_before) = self.pdf_before.ok_ref() && let Some(pdf_after) = self.pdf_after.ok_ref() {
                let image_before = pdf_to_image(pdf_before, self.width, self.page_number);
                let image_after = pdf_to_image(pdf_after, self.width, self.page_number);

                let mut xor_image = image::RgbaImage::new(
                    self.width,
                    image_before.height().max(image_after.height()),
                );

                xor_image
                    .enumerate_pixels_mut()
                    .for_each(|(x, y, pixel)| {
                        match (
                            image_before.get_pixel_checked(x, y),
                            image_after.get_pixel_checked(x, y),
                        ) {
                            (Some(a), Some(b)) => {
                                *pixel = image::Rgba([
                                    a.0[0] ^ b.0[0],
                                    a.0[1] ^ b.0[1],
                                    a.0[2] ^ b.0[2],
                                    u8::MAX,
                                ]);
                            }
                            (Some(a), None) | (None, Some(a)) => {
                                *pixel = *a;
                            }
                            (None, None) => unreachable!(),
                        }
                    });

                let mut bytes = std::io::Cursor::new(Vec::new());
                xor_image
                    .write_to(&mut bytes, image::ImageFormat::Bmp)
                    .unwrap();
                self.image_xor = Some(egui::load::Bytes::Shared(bytes.into_inner().into()));
            }
        });
    }
}

fn powered_by_egui_and_eframe(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.label("Copyright ");
        ui.hyperlink_to("Partum Engineering Pty Ltd", "https://partumengineering.com.au/");
        ui.label(".");
    });
}

fn pdf_picker(
    ui: &mut egui::Ui,
    label: impl for<'a> egui::IntoAtoms<'a>,
    pdf: &mut Bind<pdfium_render::prelude::PdfDocument<'static>, String>,
    pdfium: &'static pdfium_render::prelude::Pdfium,
) -> egui::Response {
    let mut response = ui.response();

    if ui.button(label).clicked() {
        pdf.refresh(async move {
            let handle = rfd::AsyncFileDialog::new()
                .add_filter("PDF", &["pdf"])
                .pick_file()
                .await
                .ok_or(String::from("No File Chosen"))?;

            let pdf_bytes = handle.read().await;

            pdfium.load_pdf_from_byte_vec(pdf_bytes, None).map_err(|err| err.to_string())
        });

        response.mark_changed();
    }

    if let Some(pdf) = pdf.ok_ref() {
        if let Some(tag) = pdf.metadata().get(pdfium_render::prelude::PdfDocumentMetadataTagType::Title) {
            ui.label(format!("Loaded {}", tag.value()));
        } else {
            ui.label("Loaded Untitled Document");
        }
    } else if let Some(err) = pdf.err_ref() {
        ui.label(format!("Could not load drawing: {}", err));
    } else if pdf.is_idle() {
        ui.label("Please select a drawing...");
    } else if pdf.is_pending() {
        ui.label("Loading...");
    }

    response
}

fn pdf_to_image(
    pdf: &pdfium_render::prelude::PdfDocument<'static>,
    width: u32,
    page_number: u16,
) -> image::RgbaImage {
    use pdfium_render::prelude::*;

    let page = pdf.pages().get(page_number).unwrap();
    let image = page
        .render_with_config(&PdfRenderConfig::new().set_target_width(width as i32))
        .unwrap()
        .as_image();

    image.into_rgba8()
}