export type OperationResult =
  | { Ok: any; Err?: undefined }
  | { Ok?: undefined; Err: any };
// `lru-cache` (in our dependencies) uses the global `AbortSignal` type
// which isn't readily available in our global types, though it is available
// in deno.
// https://github.com/isaacs/node-lru-cache/pull/247#issuecomment-1204481394
// https://deno.land/api@v1.29.2?s=AbortSignal
declare global {
  type AbortSignal = any;
}
