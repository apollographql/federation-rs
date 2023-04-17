declare namespace crypto {
  function getRandomValues<
    T extends
      | Int8Array
      | Int16Array
      | Int32Array
      | Uint8Array
      | Uint16Array
      | Uint32Array
      | Uint8ClampedArray
      | Float32Array
      | Float64Array
      | BigInt64Array
      | BigUint64Array
      | DataView
      | null
  >(array: T): T;
}

const rnds8 = new Uint8Array(16);

const randomValue = crypto.getRandomValues(rnds8);

if (!randomValue) {
  throw "couldn't use crypto.getRandomValues";
}
