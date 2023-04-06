export type CompositionResult =
  | { Ok: any; Err?: undefined }
  | { Ok?: undefined; Err: any };
