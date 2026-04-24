use std::future::Future;
use std::pin::Pin;

use actix_web::{dev::Payload, web, FromRequest, HttpRequest};
use serde::de::DeserializeOwned;

use crate::errors::CustomError;

pub trait Validate {
    fn validate(&self) -> Result<(), CustomError>;
}

pub struct ValidatedJson<T>(pub T);

impl<T> ValidatedJson<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> FromRequest for ValidatedJson<T>
where
    T: Validate + DeserializeOwned + 'static,
{
    type Error = actix_web::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let json_fut = web::Json::<T>::from_request(req, payload);
        Box::pin(async move {
            let json = json_fut.await?;
            let inner = json.into_inner();
            inner.validate()?;
            Ok(ValidatedJson(inner))
        })
    }
}
