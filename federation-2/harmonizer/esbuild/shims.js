globalThis.buffer_shim = require("buffer").Buffer;
globalThis.URL = require("url").URL;

// We already use a shim for the Node.js util module, but it
// unfortunately does not include TextEncoder or TextDecoder, so we add
// them here. See https://github.com/browserify/node-util/issues/46.
function fixUtilTextEncoderDecoder(util) {
  if (!util.TextEncoder || !util.TextDecoder) {
    util.TextEncoder = class TextEncoderShim {
      encode(str) {
        if (typeof str === "string") {
          return Deno.core.encode(str);
        }
      }
    };
    util.TextDecoder = class TextDecoderShim {
      decode(buf) {
        if (buf instanceof Uint8Array) {
          return Deno.core.decode(buf);
        }
      }
    };
  }
  const encoder = new util.TextEncoder();
  const decoder = new util.TextDecoder();
  // Antidisestablishmentarianism in Vietnamese:
  const testWord = "Sự-phản-đối-việc-tách-nhà-thờ-ra-khỏi-nhà-nước";
  const buffer = encoder.encode(testWord);
  if (!(buffer instanceof Uint8Array) || decoder.decode(buffer) !== testWord) {
    throw new Error("Could not polyfill util.Text{Encoder,Decoder}");
  }
  return util;
}

fixUtilTextEncoderDecoder(require("util"));
