export type CompositionResult =
  | { Ok: any; Err?: undefined }
  | { Ok?: undefined; Err: any };

export type CompositionError = {
  message?: string;
  code?: string;
  nodes: BuildErrorNode[];
  omittedNodesCount: number;
};

export type BuildErrorNode = {
  subgraph: string;
  source?: string;
  start?: Position;
  end?: Position;
};

export type Position = {
  start: number;
  end: number;
  line: number;
  column: number;
};

export type CompositionHint = {
  message: string;
  code: string;
  nodes: BuildErrorNode[];
  omittedNodesCount: number;
};
