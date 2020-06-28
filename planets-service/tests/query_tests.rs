use actix_web::{App, guard, test, web};
use jsonpath_lib as jsonpath;
use serde::{Deserialize, Serialize};
use serde_json::Map;

use planets_service::{create_schema, index, prepare_env};

const PLANET_FRAGMENT: &str = "
    fragment planetFragment on Planet {
        id
        name
        type
        details {
            meanRadius
            mass
            ... on InhabitedPlanetDetails {
                population
            }
        }
    }
";

#[actix_rt::test]
async fn test_planets() {
    let pool = prepare_env();
    let schema = create_schema(pool);

    let mut service = test::init_service(App::new()
        .data(schema.clone())
        .service(web::resource("/").guard(guard::Post()).to(index)))
        .await;

    let query = "
        {
            planets {
                id
                name
                type
                details {
                    meanRadius
                    mass
                    ... on InhabitedPlanetDetails {
                        population
                    }
                }
            }
        }
        ".to_string();

    let request_body = GraphQLCustomRequest {
        query,
        variables: Map::new(),
    };

    let request = test::TestRequest::post().uri("/").set_json(&request_body).to_request();

    let response: GraphQLCustomResponse = test::read_response_json(&mut service, request).await;

    fn get_planet_as_json(all_planets: &serde_json::Value, index: i32) -> &serde_json::Value {
        jsonpath::select(all_planets, &format!("$..planets[{}]", index)).expect("Can't get planet by JSON path")[0]
    }

    let mercury_json = get_planet_as_json(&response.data, 0);
    check_planet(mercury_json, 1, "Mercury", "TERRESTRIAL_PLANET", "2439.7");

    let earth_json = get_planet_as_json(&response.data, 2);
    check_planet(earth_json, 3, "Earth", "TERRESTRIAL_PLANET", "6371.0");

    let neptune_json = get_planet_as_json(&response.data, 7);
    check_planet(neptune_json, 8, "Neptune", "ICE_GIANT", "24622.0");
}

#[actix_rt::test]
async fn test_planet_by_id() {
    let pool = prepare_env();
    let schema = create_schema(pool);

    let mut service = test::init_service(App::new()
        .data(schema.clone())
        .service(web::resource("/").guard(guard::Post()).to(index)))
        .await;

    let query = "
        {
            planet(id: 3) {
                ... planetFragment
            }
        }
        ".to_string() + PLANET_FRAGMENT;

    let request_body = GraphQLCustomRequest {
        query,
        variables: Map::new(),
    };

    let request = test::TestRequest::post().uri("/").set_json(&request_body).to_request();

    let response: GraphQLCustomResponse = test::read_response_json(&mut service, request).await;

    let earth_json = jsonpath::select(&response.data, "$..planet").expect("Can't get planet by JSON path")[0];
    check_planet(earth_json, 3, "Earth", "TERRESTRIAL_PLANET", "6371.0");
}

#[actix_rt::test]
async fn test_variable() {
    let pool = prepare_env();
    let schema = create_schema(pool);

    let mut service = test::init_service(App::new()
        .data(schema.clone())
        .service(web::resource("/").guard(guard::Post()).to(index)))
        .await;

    let query = "
        query testPlanetById($planetId: String!) {
            planet(id: $planetId) {
                ... planetFragment
            }
        }".to_string() + PLANET_FRAGMENT;

    let jupiter_id = 5;
    let mut variables = Map::new();
    variables.insert("planetId".to_string(), jupiter_id.into());

    let request_body = GraphQLCustomRequest {
        query,
        variables,
    };

    let request = test::TestRequest::post().uri("/").set_json(&request_body).to_request();

    let response: GraphQLCustomResponse = test::read_response_json(&mut service, request).await;

    let jupiter_json = jsonpath::select(&response.data, "$..planet").expect("Can't get planet by JSON path")[0];
    check_planet(jupiter_json, 5, "Jupiter", "GAS_GIANT", "69911.0");
}

fn check_planet(planet_json: &serde_json::Value, id: i32, name: &str, planet_type: &str, mean_radius: &str) {
    fn check_property(planet_json: &serde_json::Value, property_name: &str, property_expected_value: &str) {
        let json_path = format!("$..{}", property_name);
        assert_eq!(property_expected_value, jsonpath::select(&planet_json, &json_path).expect("Can't get property")[0].as_str().expect("Can't get property as str"));
    }
    check_property(planet_json, "id", &id.to_string());
    check_property(planet_json, "name", name);
    check_property(planet_json, "type", planet_type);
    check_property(planet_json, "details.meanRadius", mean_radius);
}

#[derive(Serialize)]
struct GraphQLCustomRequest {
    query: String,
    variables: Map<String, serde_json::Value>,
}

#[derive(Deserialize)]
struct GraphQLCustomResponse {
    data: serde_json::Value,
}
