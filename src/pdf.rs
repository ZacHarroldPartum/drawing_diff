use core::{
    fmt::{Debug, Display},
    ops::Deref,
};
use std::sync::LazyLock;

use pdfium_render::prelude::{
    PdfDocument, PdfDocumentMetadataTagType, PdfRenderConfig, Pdfium, PdfiumError,
};

static PDFIUM: LazyLock<Pdfium> = LazyLock::new(Pdfium::default);

pub(crate) struct Pdf<'a> {
    inner: PdfDocument<'a>,
    title: Option<String>,
    pages: u16,
}

impl<'a> Pdf<'a> {
    fn new(inner: PdfDocument<'a>) -> Self {
        let title = inner
            .metadata()
            .get(PdfDocumentMetadataTagType::Title)
            .map(|tag| tag.value().to_owned());
        let pages = inner.pages().len();

        Self {
            inner,
            title,
            pages,
        }
    }

    pub(crate) fn title(&self) -> &Option<impl Deref<Target = str> + Debug + Display> {
        &self.title
    }

    pub(crate) fn pages(&self) -> u16 {
        self.pages
    }

    pub(crate) fn render(
        &self,
        width: u32,
        page_number: u16,
    ) -> Result<image::RgbaImage, PdfiumError> {
        let image = self
            .inner
            .pages()
            .get(page_number)?
            .render_with_config(&PdfRenderConfig::new().set_target_width(width as i32))?
            .as_image()
            .into_rgba8();

        Ok(image)
    }
}

impl TryFrom<Vec<u8>> for Pdf<'static> {
    type Error = PdfiumError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        let inner = PDFIUM.load_pdf_from_byte_vec(value, None)?;
        Ok(Self::new(inner))
    }
}

impl<'a> TryFrom<&'a [u8]> for Pdf<'a> {
    type Error = PdfiumError;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        let inner = PDFIUM.load_pdf_from_byte_slice(value, None)?;
        Ok(Self::new(inner))
    }
}
