import PDFiumModule from "./assets/pdfium.esm.js";

export default function myInitializer () {
  return {
    onStart: () => {
      console.log("Loading...");
      console.time("trunk-initializer");
    },
    onProgress: ({current, total}) => {
      if (!total) {
        console.log("Loading...", current, "bytes");
      } else {
        console.log("Loading...", Math.round((current/total) * 100), "%" )
      }
    },
    onComplete: () => {
      console.log("Loading... done!");
      console.timeEnd("trunk-initializer");
    },
    onSuccess: (wasm) => {
      console.log("Loading... successful!");

      PDFiumModule().then(async pdfiumModule => {
        wasm.initialize_pdfium_render(
            pdfiumModule, // Emscripten-wrapped Pdfium WASM module
            wasm, // wasm_bindgen-wrapped WASM module built from our Rust application
            false, // Debugging flag; set this to true to get tracing information logged to the Javascript console
        );

        wasm.start();
      });
    },
    onFailure: (error) => {
      console.warn("Loading... failed!", error);
    }
  }
};