import { composeServices } from "@apollo/federation";
import { parse, Token } from "graphql";
import {
  BuildErrorNode,
  CompositionError,
  CompositionResult,
  Position,
} from "./types";

export function composition(
  serviceList: { sdl: string; name: string; url: string }[]
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

  let subgraphList = serviceList.map(({ sdl, ...rest }) => ({
    typeDefs: parseTypedefs(sdl),
    ...rest,
  }));

  const composed = composeServices(subgraphList);
  if (composed.errors) {
    //We need to reshape the errors
    let errors: CompositionError[] = [];
    composed.errors.map((err) => {
      let nodes: BuildErrorNode[] = [];
      err.nodes.map((node) => {
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
        hints: [],
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
function parseTypedefs(source: string) {
  try {
    return parse(source);
  } catch (err) {
    // Return the error in a way that we know how to handle it.
    done({ Err: [{ message: err.toString() }] });
  }
}
