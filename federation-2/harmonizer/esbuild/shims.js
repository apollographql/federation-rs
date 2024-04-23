globalThis.buffer_shim = require("buffer").Buffer;
globalThis.URL = require("url").URL;

// We already use a polyfill for the Node.js util module, but it unfortunately
// does not include TextEncoder or TextDecoder, so we add them here. See
// https://github.com/browserify/node-util/issues/46.
const util = require("util");
if (!util.TextEncoder || !util.TextDecoder) {
  require("fastestsmallesttextencoderdecoder");
  if (
    !(util.TextEncoder = globalThis.TextEncoder) ||
    !(util.TextDecoder = globalThis.TextDecoder)
  ) {
    throw new Error("Could not polyfill util.Text{Encoder,Decoder}");
  }
}
