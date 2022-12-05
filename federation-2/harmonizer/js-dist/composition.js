Object.defineProperty(exports, "__esModule", { value: true });
exports.composition = void 0;
const composition_1 = require("@apollo/composition");
const graphql_1 = require("graphql");
function composition(serviceList) {
    if (!serviceList || !Array.isArray(serviceList)) {
        throw new Error("Error in JS-Rust-land: serviceList missing or incorrect.");
    }
    serviceList.some((service) => {
        if (typeof service.name !== "string" ||
            !service.name ||
            (typeof service.url !== "string" && service.url) ||
            (typeof service.sdl !== "string" && service.sdl)) {
            throw new Error("Missing required data structure on service.");
        }
    });
    let subgraphList = serviceList.map(({ sdl, ...rest }) => ({
        typeDefs: parseTypedefs(sdl),
        ...rest,
    }));
    return (0, composition_1.composeServices)(subgraphList);
}
exports.composition = composition;
function parseTypedefs(source) {
    try {
        return (0, graphql_1.parse)(source);
    }
    catch (err) {
        done({ Err: [{ message: err.toString() }] });
    }
}
//# sourceMappingURL=composition.js.map