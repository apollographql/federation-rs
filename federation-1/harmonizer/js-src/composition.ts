import { composeServices } from "@apollo/federation";
import { parse } from "graphql";

export function composition(
  serviceList: { sdl: string; name: string; url: string }[]
) {
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

  return composeServices(subgraphList);
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
