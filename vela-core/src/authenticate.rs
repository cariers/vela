use std::collections::HashMap;

use futures::{AsyncRead, AsyncWrite};
use volans::core::{InboundUpgrade, UpgradeInfo};

pub struct Authentication {
    pub resource: String,
    pub token: String,
    pub metadata: HashMap<String, String>,
}

pub struct JwtInboundUpgrade {
    protocol: &'static str,
    secret_key: String,
}

impl UpgradeInfo for JwtInboundUpgrade {
    type Info = &'static str;
    type InfoIter = std::iter::Once<Self::Info>;

    fn protocol_info(&self) -> Self::InfoIter {
        std::iter::once(self.protocol)
    }
}

// impl<S> InboundUpgrade<S> for JwtInboundUpgrade
// where
//     S: AsyncWrite + AsyncRead + Unpin,
// {
//     type Output = (, S, );
//     fn upgrade_inbound(self, socket: S, info: Self::Info) -> Self::Future {

//     }
// }

pub struct Claims {}
