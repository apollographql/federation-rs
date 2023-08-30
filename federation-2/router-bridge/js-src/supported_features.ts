import {
  AUTHENTICATED_VERSIONS,
  DEFAULT_SUPPORTED_SUPERGRAPH_FEATURES,
  FeatureVersion,
  REQUIRES_SCOPES_VERSIONS,
} from "@apollo/federation-internals";

const authenticatedV01Feature = AUTHENTICATED_VERSIONS.find(
  new FeatureVersion(0, 1)
);
if (!authenticatedV01Feature) {
  throw Error(
    "Federation package unexpectedly did not contain authenticated v0.1 spec."
  );
}
const requiresScopesV01Feature = REQUIRES_SCOPES_VERSIONS.find(
  new FeatureVersion(0, 1)
);
if (!requiresScopesV01Feature) {
  throw new Error(
    "Federation package unexpectedly did not contain requiresScopes v0.1 spec."
  );
}
export const ROUTER_SUPPORTED_SUPERGRAPH_FEATURES: Set<String> = new Set(
  DEFAULT_SUPPORTED_SUPERGRAPH_FEATURES
)
  .add(authenticatedV01Feature.toString())
  .add(requiresScopesV01Feature.toString());
