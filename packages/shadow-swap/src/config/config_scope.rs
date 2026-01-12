use actix_web::web;

use crate::api::routes::{
    convert_amount, get_all_prices, get_intent_status, get_metrics, get_price, get_stats,
    health_check, indexer_event, initiate_bridge, list_intents, root,
};

pub fn configure(conf: &mut web::ServiceConfig) {
    let scope = web::scope("/api/v1")
        .service(web::resource("/bridge/initiate").route(web::post().to(initiate_bridge)))
        .service(get_intent_status)
        .service(list_intents)
        .service(indexer_event)
        .service(get_price)
        .service(get_all_prices)
        .service(convert_amount)
        .service(get_metrics)
        .service(get_stats)
        .service(health_check)
        .service(root);

    conf.service(scope);
}
