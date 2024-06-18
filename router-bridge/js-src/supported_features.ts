import {
  AUTHENTICATED_VERSIONS,
  DEFAULT_SUPPORTED_SUPERGRAPH_FEATURES,
  FeatureDefinition,
  FeatureDefinitions,
  POLICY_VERSIONS,
  REQUIRES_SCOPES_VERSIONS,
  SOURCE_VERSIONS,
  CONTEXT_VERSIONS,
} from "@apollo/federation-internals";

export const ROUTER_SUPPORTED_SUPERGRAPH_FEATURES: Set<string> = new Set(
  DEFAULT_SUPPORTED_SUPERGRAPH_FEATURES
);

function addToRouterFeatures<T extends FeatureDefinition>(
  definitions: FeatureDefinitions<T>
) {
  definitions.versions().forEach((version) => {
    const feature = definitions.find(version);
    if (!feature) {
      throw Error(
        `Federation package unexpectedly did not contain feature spec ${definitions.identity}/${version}`
      );
    }
    ROUTER_SUPPORTED_SUPERGRAPH_FEATURES.add(feature.toString());
  });
}

addToRouterFeatures(AUTHENTICATED_VERSIONS);
addToRouterFeatures(REQUIRES_SCOPES_VERSIONS);
addToRouterFeatures(POLICY_VERSIONS);
addToRouterFeatures(SOURCE_VERSIONS);
addToRouterFeatures(CONTEXT_VERSIONS);
