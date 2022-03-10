/** @typedef {{sdl: string, name: string, url?: string;}} ServiceDefinition */
/** @typedef {{message: string;}} BuildHint */

/**
 * This `composition` is defined as a global by the runtime we define in Rust.
 * We declare this as a `var` here only to allow the TSDoc type annotation to be
 * applied to it. Running `var` multiple times has no effect.
 * @type {{
 *   composeServices: import('@apollo/composition').composeServices,
 *   parseGraphqlDocument: import('graphql').parse
 * }} */
var composition;

/**
 * @type {ServiceDefinition[]}
 */
var serviceList = serviceList;

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

serviceList = serviceList.map(({ sdl, ...rest }) => ({
  typeDefs: parseTypedefs(sdl),
  ...rest,
}));

function parseTypedefs(source) {
  try {
    return composition.parseGraphqlDocument(source);
  } catch (err) {
    // Return the error in a way that we know how to handle it.
    done({ Err: [{ message: err.toString() }] });
  }
}

try {
  // /**
  //  * @type {{ errors: Error[], supergraphSdl?: undefined, hints: undefined } | { errors?: undefined, supergraphSdl: string, hints: string }}
  //  */
  const composed = composition.composeServices(serviceList);
  /**
   * @type {BuildHint[]}
   */
  let hints = [];
  if (composed.hints) {
    composed.hints.map((composed_hint) => {
      hints.push({ message: composed_hint.toString() });
    });
  }
  done(
    composed.errors
      ? { Err: composed.errors }
      : {
          Ok: {
            supergraphSdl: composed.supergraphSdl,
            hints,
          },
        }
  );
} catch (err) {
  done({ Err: [{ message: err.toString() }] });
}
