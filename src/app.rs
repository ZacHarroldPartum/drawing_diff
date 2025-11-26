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
    height: u32,
    view: View,

    #[serde(skip)]
    page_number: u16,
    #[serde(skip)]
    page_count: u16,
    #[serde(skip)]
    pdf_before: Bind<crate::pdf::Pdf<'static>, String>,
    #[serde(skip)]
    pdf_after: Bind<crate::pdf::Pdf<'static>, String>,

    #[serde(skip)]
    images: Option<Images>,

    #[serde(skip)]
    scene_rect: egui::Rect,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            width: 2048,
            height: 2048,
            view: View::default(),
            page_number: 0,
            page_count: 0,
            pdf_before: Bind::default(),
            pdf_after: Bind::default(),
            images: None,
            scene_rect: egui::Rect::ZERO,
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

    changed |= pdf_picker(ui, "Before…", &mut app.pdf_before).changed();
    changed |= pdf_picker(ui, "After…", &mut app.pdf_after).changed();

    ui.horizontal(|ui| {
        ui.radio_value(&mut app.view, View::Before, "Before");
        ui.radio_value(&mut app.view, View::Added, "Added");
        ui.radio_value(&mut app.view, View::Removed, "Removed");
        ui.radio_value(&mut app.view, View::After, "After");
    });

    if let Some(pdf_before) = app.pdf_before.ok_ref()
        && let Some(pdf_after) = app.pdf_after.ok_ref()
    {
        app.page_count = pdf_before.pages().min(pdf_after.pages());

        let last_page = app.page_count.saturating_sub(1);
        app.page_number = app.page_number.clamp(0, last_page);
        if last_page > 0 {
            changed |= ui
                .add(egui::Slider::new(&mut app.page_number, 0..=last_page).text("Page"))
                .changed();
        }
    }

    if changed {
        app.images = None;

        ctx.forget_image(URI_IMAGE_BEFORE);
        ctx.forget_image(URI_IMAGE_ADDED);
        ctx.forget_image(URI_IMAGE_REMOVED);
        ctx.forget_image(URI_IMAGE_AFTER);
    }

    ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
        powered_by_egui_and_eframe(ui);
        egui::warn_if_debug_build(ui);
    });
}

fn central_panel(app: &mut TemplateApp, _ctx: &egui::Context, ui: &mut egui::Ui) {
    // The central panel the region left after adding TopPanel's and SidePanel's
    if let Some(images) = app.images.as_ref() {
        egui::containers::Scene::new().zoom_range(0.75..=16.0).show(
            ui,
            &mut app.scene_rect,
            |ui| {
                ui.centered_and_justified(|ui| match app.view {
                    View::Before => ui.add(egui::Image::from_bytes(
                        URI_IMAGE_BEFORE,
                        images.before.clone(),
                    )),
                    View::Added => ui.add(egui::Image::from_bytes(
                        URI_IMAGE_ADDED,
                        images.added.clone(),
                    )),
                    View::Removed => ui.add(egui::Image::from_bytes(
                        URI_IMAGE_REMOVED,
                        images.removed.clone(),
                    )),
                    View::After => ui.add(egui::Image::from_bytes(
                        URI_IMAGE_AFTER,
                        images.after.clone(),
                    )),
                });
            },
        );
    } else if let Some(pdf_before) = app.pdf_before.ok_ref()
        && let Some(pdf_after) = app.pdf_after.ok_ref()
    {
        app.width = ui.available_width().round() as u32 * 2;
        let image_before = pdf_before.render(app.width, app.page_number).unwrap();
        let image_after = pdf_after.render(app.width, app.page_number).unwrap();
        app.height = image_before.height().max(image_after.height());
        app.images = Some(Images::generate(image_before, image_after));
    }
}

struct Images {
    before: Bytes,
    after: Bytes,
    added: Bytes,
    removed: Bytes,
}

impl Images {
    fn generate(before: image::RgbaImage, after: image::RgbaImage) -> Self {
        let width = before.width().max(after.width());
        let height = before.height().max(after.height());
        let mut added = image::RgbaImage::new(width, height);
        let mut removed = image::RgbaImage::new(width, height);

        added
            .enumerate_pixels_mut()
            .zip(removed.enumerate_pixels_mut())
            .for_each(|((x, y, pixel_added), (_, _, pixel_removed))| {
                match (
                    before.get_pixel_checked(x, y),
                    after.get_pixel_checked(x, y),
                ) {
                    (Some(a), Some(b)) => {
                        let metric = (a.0[0] ^ b.0[0])
                            .saturating_add(a.0[1] ^ b.0[1])
                            .saturating_add(a.0[2] ^ b.0[2]);

                        if metric > 0 {
                            *pixel_added = *b;
                            *pixel_removed = *a;
                        } else {
                            *pixel_added = image::Rgba([u8::MAX; 4]);
                            *pixel_removed = image::Rgba([u8::MAX; 4]);
                        };
                    }
                    (Some(a), None) | (None, Some(a)) => {
                        *pixel_added = *a;
                        *pixel_removed = *a;
                    }
                    (None, None) => unreachable!(),
                }
            });

        let mut bytes = std::io::Cursor::new(Vec::new());
        before
            .write_to(&mut bytes, image::ImageFormat::Bmp)
            .expect("writing to buffer must succeed");
        let before = egui::load::Bytes::Shared(bytes.into_inner().into());

        let mut bytes = std::io::Cursor::new(Vec::new());
        added
            .write_to(&mut bytes, image::ImageFormat::Bmp)
            .expect("writing to buffer must succeed");
        let added = egui::load::Bytes::Shared(bytes.into_inner().into());

        let mut bytes = std::io::Cursor::new(Vec::new());
        removed
            .write_to(&mut bytes, image::ImageFormat::Bmp)
            .expect("writing to buffer must succeed");
        let removed = egui::load::Bytes::Shared(bytes.into_inner().into());

        let mut bytes = std::io::Cursor::new(Vec::new());
        after
            .write_to(&mut bytes, image::ImageFormat::Bmp)
            .expect("writing to buffer must succeed");
        let after = egui::load::Bytes::Shared(bytes.into_inner().into());

        Self {
            before,
            after,
            added,
            removed,
        }
    }
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
    pdf: &mut Bind<crate::pdf::Pdf<'static>, String>,
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

            crate::pdf::Pdf::try_from(pdf_bytes).map_err(|err| err.to_string())
        });

        response.mark_changed();
    }

    match pdf.state() {
        egui_async::StateWithData::Idle => {
            ui.label("Please select a drawing...");
        }
        egui_async::StateWithData::Pending => {
            ui.add(egui::widgets::Spinner::new());
        }
        egui_async::StateWithData::Finished(pdf) => {
            if let Some(title) = pdf.title() {
                ui.label(format!("Loaded {title}"));
            } else {
                ui.label("Loaded Untitled Document");
            }
        }
        egui_async::StateWithData::Failed(err) => {
            ui.label(format!("Could not load drawing: {err}"));
        }
    }

    response
}

#[derive(PartialEq, Default, serde::Deserialize, serde::Serialize)]
enum View {
    Before,
    #[default]
    Added,
    Removed,
    After,
}
