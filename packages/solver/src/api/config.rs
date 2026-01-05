use actix_web::web;

use crate::api::routes::{get_status, health_check, metrics, ready};

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1")
            .service(health_check)
            .service(metrics)
            .service(get_status)
            .service(ready),
    );
}
