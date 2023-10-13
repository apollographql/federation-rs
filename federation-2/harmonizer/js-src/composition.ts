import { composeServices } from "@apollo/composition";
import { ASTNode, GraphQLError, parse, Token } from "graphql";
import {
  BuildErrorNode,
  CompositionError,
  CompositionHint,
  CompositionResult,
  Position,
} from "./types";
import { ERRORS } from "@apollo/federation-internals";

const NODES_SIZE_LIMIT: number = 20;

export function composition(
  serviceList: { sdl: string; name: string; url?: string }[],
): CompositionResult {
  if (!serviceList || !Array.isArray(serviceList)) {
    throw new Error("Error in JS-Rust-land: serviceList missing or incorrect.");
  }

  serviceList.some((service) => {
    if (
      typeof service.name !== "string" ||
      !service.name ||
      (typeof service.url !== "string" && service.url) ||
      (typeof service.sdl !== "string" && service.sdl)
    ) {
      throw new Error("Missing required data structure on service.");
    }
  });

  let subgraphList = serviceList.map(({ sdl, name, ...rest }) => ({
    typeDefs: parseTypedefs(sdl, name),
    name,
    ...rest,
  }));

  const composed = composeServices(subgraphList);
  let hints: CompositionHint[] = [];
  if (composed.hints) {
    composed.hints.map((composed_hint) => {
      let nodes: BuildErrorNode[] = [];

      let omittedNodesCount = 0;
      // for issues that happen in all subgraphs and with a large amount of subgraphs,
      // only add nodes up to the limit to prevent massive responses
      // (OOM errors when going from js to rust)
      if (composed_hint.nodes?.length >= NODES_SIZE_LIMIT) {
        composed_hint.nodes
          ?.slice(0, NODES_SIZE_LIMIT)
          .map((node) => nodes.push(getBuildErrorNode(node)));
        omittedNodesCount = composed_hint.nodes?.length - NODES_SIZE_LIMIT;
      } else {
        composed_hint.nodes?.map((node) => nodes.push(getBuildErrorNode(node)));
      }

      hints.push({
        message: composed_hint.toString(),
        code: composed_hint.definition.code,
        nodes,
        omittedNodesCount: omittedNodesCount,
      });
    });
  }

  if (composed.errors) {
    //We need to reshape the errors
    let errors: CompositionError[] = [];
    composed.errors.map((err) => {
      let nodes: BuildErrorNode[] = [];

      let omittedNodesCount = 0;
      // for issues that happen in all subgraphs and with a large amount of subgraphs,
      // only add nodes up to the limit to prevent massive responses
      // (OOM errors when going from js to rust)
      if (err.nodes?.length >= NODES_SIZE_LIMIT) {
        err.nodes
          ?.slice(0, NODES_SIZE_LIMIT)
          .map((node) => nodes.push(getBuildErrorNode(node)));
        omittedNodesCount = err.nodes?.length - NODES_SIZE_LIMIT;
      } else {
        err.nodes?.map((node) => nodes.push(getBuildErrorNode(node)));
      }

      errors.push({
        code: (err?.extensions["code"] as string) ?? "",
        message: err.message,
        nodes,
        omittedNodesCount: omittedNodesCount,
      });
    });

    return { Err: errors };
  } else
    return {
      Ok: {
        supergraphSdl: composed.supergraphSdl,
        hints,
      },
    };
}

function getBuildErrorNode(node: ASTNode) {
  let n: BuildErrorNode = {
    subgraph: (node as any)?.subgraph,
  };
  if (node.loc) {
    n.source = node.loc?.source?.body;
    n.start = getPosition(node.loc.startToken);
    n.end = getPosition(node.loc.endToken);
  }
  return n;
}

function getPosition(token: Token): Position {
  return {
    start: token.start,
    end: token.end,
    line: token.line,
    column: token.column,
  };
}

//@ts-ignore
function parseTypedefs(source: string, subgraphName: string) {
  try {
    return parse(source);
  } catch (err) {
    let nodeTokens: BuildErrorNode[] = [];
    if (err instanceof GraphQLError) {
      nodeTokens =
        err.nodes != null
          ? err.nodes.map(function (n: ASTNode) {
              return {
                subgraph: subgraphName,
                source: source,
                start: getPosition(n.loc.startToken),
                end: getPosition(n.loc.endToken),
              };
            })
          : [
              {
                subgraph: subgraphName,
                source: source,
              },
            ];
    }

    // Return the error in a way that we know how to handle it.
    done({
      Err: [
        {
          code: ERRORS.INVALID_GRAPHQL.code,
          message: "[" + subgraphName + "] - " + err.toString(),
          nodes: nodeTokens,
        },
      ],
    });
  }
}
