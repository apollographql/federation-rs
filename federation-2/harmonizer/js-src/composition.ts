import { composeServices } from "@apollo/composition";
import { parse, Token } from "graphql";
import {
  BuildErrorNode,
  CompositionError,
  CompositionResult,
  Position,
} from "./types";

export function composition(
  serviceList: { sdl: string; name: string; url?: string }[]
): CompositionResult {
  if (!serviceList || !Array.isArray(serviceList)) {
    throw new Error("Error in JS-Rust-land: serviceList missing or incorrect.");
  }

  let subgraphList = serviceList.map(({ sdl, ...rest }) => ({
    typeDefs: parseTypedefs(sdl),
    ...rest,
  }));

  const composed = composeServices(subgraphList);
  let hints: { message: string }[] = [];
  if (composed.hints) {
    composed.hints.map((composed_hint) => {
      hints.push({ message: composed_hint.toString() });
    });
  }

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
function parseTypedefs(source: string) {
  try {
    return parse(source);
  } catch (err) {
    // Return the error in a way that we know how to handle it.
    done({ Err: [{ message: err.toString() }] });
  }
}
