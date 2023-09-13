import {
  buildSchema,
  GraphQLError,
  parse,
  Source,
  validate as validateGraphQL,
} from "graphql";

export function validate(
  schema: string,
  query: string
): ReadonlyArray<GraphQLError> {
  let ts = buildSchema(schema);
  let op = parse(new Source(query, "op.graphql"));
  return validateGraphQL(ts, op);
}
