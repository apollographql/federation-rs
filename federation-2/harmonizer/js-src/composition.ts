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

export function composition(
  serviceList: { sdl: string; name: string; url?: string }[]
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
      composed_hint.nodes?.map((node) => {
        nodes.push({
          subgraph: (node as any)?.subgraph,
          source: node?.loc.source.body,
          start: getPosition(node.loc.startToken),
          end: getPosition(node.loc.endToken),
        });
      });

      hints.push({
        message: composed_hint.toString(),
        code: composed_hint.definition.code,
        nodes,
      });
    });
  }

  if (composed.errors) {
    //We need to reshape the errors
    let errors: CompositionError[] = [];
    composed.errors.map((err) => {
      let nodes: BuildErrorNode[] = [];
      err.nodes?.map((node) => {
        nodes.push({
          subgraph: (node as any)?.subgraph,
          source: node?.loc.source.body,
          start: getPosition(node.loc.startToken),
          end: getPosition(node.loc.endToken),
        });
      });

      errors.push({
        code: (err?.extensions["code"] as string) ?? "",
        message: err.message,
        nodes,
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
