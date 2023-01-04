use criterion::criterion_group;
use criterion::criterion_main;
use criterion::Criterion;
use router_bridge::planner::Planner;
use router_bridge::planner::QueryPlannerConfig;

const QUERY: &str = include_str!("query.graphql");
const SCHEMA: &str = include_str!("schema.graphql");

fn from_elem(c: &mut Criterion) {
    c.bench_function("query_planning", move |b| {
        let runtime = tokio::runtime::Runtime::new().unwrap();

        let planner = runtime.block_on(async {
            Planner::<serde_json::Value>::new(SCHEMA.to_string(), QueryPlannerConfig::default())
                .await
                .unwrap()
        });

        b.to_async(runtime).iter(|| async {
            planner
                .plan(QUERY.to_string(), None)
                .await
                .unwrap()
                .into_result()
                .unwrap();
        });
    });
}

criterion_group!(benches, from_elem);
criterion_main!(benches);
