use egui::load::Bytes;
use egui_async::{Bind, EguiAsyncPlugin};

const URI_IMAGE_BEFORE: &str = "bytes://before.bmp";
const URI_IMAGE_ADDED: &str = "bytes://added.bmp";
const URI_IMAGE_REMOVED: &str = "bytes://removed.bmp";
const URI_IMAGE_AFTER: &str = "bytes://after.bmp";

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    width: u32,
    view: View,

    page_number: u16,
    #[serde(skip)]
    pdf_before: Bind<pdfium_render::prelude::PdfDocument<'static>, String>,
    #[serde(skip)]
    pdf_after: Bind<pdfium_render::prelude::PdfDocument<'static>, String>,

    #[serde(skip)]
    image_before: Option<Bytes>,
    #[serde(skip)]
    image_removed: Option<Bytes>,
    #[serde(skip)]
    image_added: Option<Bytes>,
    #[serde(skip)]
    image_after: Option<Bytes>,

    #[serde(skip)]
    pdfium: &'static pdfium_render::prelude::Pdfium,
    // #[serde(skip)]
    // xor_image: Option<egui::load::Bytes>,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            width: 2048,
            view: View::default(),
            page_number: 0,
            pdf_before: Bind::default(),
            pdf_after: Bind::default(),
            image_before: None,
            image_removed: None,
            image_added: None,
            image_after: None,
            pdfium: Box::leak(Box::default()),
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
            side_panel(self, ctx, ui);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            central_panel(self, ctx, ui);
        });
    }
}

fn side_panel(app: &mut TemplateApp, ctx: &egui::Context, ui: &mut egui::Ui) {
    ui.heading("Drawing Diff");

    ui.separator();

    let mut changed = false;

    changed |= pdf_picker(ui, "Before…", &mut app.pdf_before, app.pdfium).changed();
    changed |= pdf_picker(ui, "After…", &mut app.pdf_after, app.pdfium).changed();

    ui.horizontal(|ui| {
        ui.radio_value(&mut app.view, View::Before, "Before");
        ui.radio_value(&mut app.view, View::Added, "Added");
        ui.radio_value(&mut app.view, View::Removed, "Removed");
        ui.radio_value(&mut app.view, View::After, "After");
    });

    if let Some(pdf_before) = app.pdf_before.ok_ref()
        && let Some(pdf_after) = app.pdf_after.ok_ref()
    {
        let page_count = pdf_before.pages().len().min(pdf_after.pages().len());
        let last_page = page_count.saturating_sub(1);
        app.page_number = app.page_number.clamp(0, last_page);
        if page_count > 1 {
            changed |= ui
                .add(egui::Slider::new(&mut app.page_number, 0..=last_page).text("Page"))
                .changed();
        }
    }

    if changed {
        app.image_before = None;
        ctx.forget_image(URI_IMAGE_BEFORE);

        app.image_added = None;
        ctx.forget_image(URI_IMAGE_ADDED);

        app.image_removed = None;
        ctx.forget_image(URI_IMAGE_REMOVED);

        app.image_after = None;
        ctx.forget_image(URI_IMAGE_AFTER);
    }

    ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
        powered_by_egui_and_eframe(ui);
        egui::warn_if_debug_build(ui);
    });
}

fn central_panel(app: &mut TemplateApp, _ctx: &egui::Context, ui: &mut egui::Ui) {
    // The central panel the region left after adding TopPanel's and SidePanel's
    if let Some(image_before) = &app.image_before
        && let Some(image_added) = &app.image_added
        && let Some(image_removed) = &app.image_removed
        && let Some(image_after) = &app.image_after
    {
        ui.centered_and_justified(|ui| match app.view {
            View::Before => ui.add(egui::Image::from_bytes(
                URI_IMAGE_BEFORE,
                image_before.clone(),
            )),
            View::Added => ui.add(egui::Image::from_bytes(
                URI_IMAGE_ADDED,
                image_added.clone(),
            )),
            View::Removed => ui.add(egui::Image::from_bytes(
                URI_IMAGE_REMOVED,
                image_removed.clone(),
            )),
            View::After => ui.add(egui::Image::from_bytes(
                URI_IMAGE_AFTER,
                image_after.clone(),
            )),
        });
    } else if app.pdf_before.is_ok() && app.pdf_after.is_ok() {
        update_diffs(app);
    }
}

fn update_diffs(app: &mut TemplateApp) {
    let Some(pdf_before) = app.pdf_before.ok_ref() else {
        return;
    };
    let Some(pdf_after) = app.pdf_after.ok_ref() else {
        return;
    };

    let image_before = pdf_to_image(pdf_before, app.width, app.page_number);
    let image_after = pdf_to_image(pdf_after, app.width, app.page_number);

    let mut image_added =
        image::RgbaImage::new(app.width, image_before.height().max(image_after.height()));

    let mut image_removed =
        image::RgbaImage::new(app.width, image_before.height().max(image_after.height()));

    image_added
        .enumerate_pixels_mut()
        .for_each(|(x, y, pixel)| {
            match (
                image_before.get_pixel_checked(x, y),
                image_after.get_pixel_checked(x, y),
            ) {
                (Some(a), Some(b)) => {
                    let metric =
                        a.0.iter()
                            .zip(b.0)
                            .take(3)
                            .map(|(a, b)| a ^ b)
                            .fold(0u8, |sum, x| sum.saturating_add(x));

                    *pixel = if metric > 0 {
                        *b
                    } else {
                        image::Rgba([u8::MAX; 4])
                    };
                }
                (Some(a), None) | (None, Some(a)) => {
                    *pixel = *a;
                }
                (None, None) => unreachable!(),
            }
        });

    image_removed
        .enumerate_pixels_mut()
        .for_each(|(x, y, pixel)| {
            match (
                image_before.get_pixel_checked(x, y),
                image_after.get_pixel_checked(x, y),
            ) {
                (Some(a), Some(b)) => {
                    let metric =
                        a.0.iter()
                            .zip(b.0)
                            .take(3)
                            .map(|(a, b)| a ^ b)
                            .fold(0u8, |sum, x| sum.saturating_add(x));

                    *pixel = if metric > 0 {
                        *a
                    } else {
                        image::Rgba([u8::MAX; 4])
                    };
                }
                (Some(a), None) | (None, Some(a)) => {
                    *pixel = *a;
                }
                (None, None) => unreachable!(),
            }
        });

    let mut bytes = std::io::Cursor::new(Vec::new());
    image_before
        .write_to(&mut bytes, image::ImageFormat::Bmp)
        .expect("writing to buffer must succeed");
    app.image_before = Some(egui::load::Bytes::Shared(bytes.into_inner().into()));

    let mut bytes = std::io::Cursor::new(Vec::new());
    image_added
        .write_to(&mut bytes, image::ImageFormat::Bmp)
        .expect("writing to buffer must succeed");
    app.image_added = Some(egui::load::Bytes::Shared(bytes.into_inner().into()));

    let mut bytes = std::io::Cursor::new(Vec::new());
    image_removed
        .write_to(&mut bytes, image::ImageFormat::Bmp)
        .expect("writing to buffer must succeed");
    app.image_removed = Some(egui::load::Bytes::Shared(bytes.into_inner().into()));

    let mut bytes = std::io::Cursor::new(Vec::new());
    image_after
        .write_to(&mut bytes, image::ImageFormat::Bmp)
        .expect("writing to buffer must succeed");
    app.image_after = Some(egui::load::Bytes::Shared(bytes.into_inner().into()));
}

fn powered_by_egui_and_eframe(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.label("Copyright ");
        ui.hyperlink_to(
            "Partum Engineering Pty Ltd",
            "https://partumengineering.com.au/",
        );
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

            pdfium
                .load_pdf_from_byte_vec(pdf_bytes, None)
                .map_err(|err| err.to_string())
        });

        response.mark_changed();
    }

    if let Some(pdf) = pdf.ok_ref() {
        if let Some(tag) = pdf
            .metadata()
            .get(pdfium_render::prelude::PdfDocumentMetadataTagType::Title)
        {
            ui.label(format!("Loaded {}", tag.value()));
        } else {
            ui.label("Loaded Untitled Document");
        }
    } else if let Some(err) = pdf.err_ref() {
        ui.label(format!("Could not load drawing: {err}"));
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

    let page = pdf.pages().get(page_number).expect("page number in bounds");
    let image = page
        .render_with_config(&PdfRenderConfig::new().set_target_width(width as i32))
        .expect("rendering must succeed")
        .as_image();

    image.into_rgba8()
}

#[derive(PartialEq, Default, serde::Deserialize, serde::Serialize)]
enum View {
    Before,
    #[default]
    Added,
    Removed,
    After,
}
