use serde::{ Deserialize, Serialize };

/// The type of query event
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[allow(missing_docs)]
pub enum QueryKind {
    User,
    Room,
    Any,
}

/// The type of third-party event
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[allow(missing_docs)]
pub enum ThirdPartyKind {
    LocationForProtocol,
    LocationForRoomAlias,
    GetProtocol,
    UserForProtocol,
    UserForUserId,
    Any,
}

/// The type of appservice event
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[allow(missing_docs)]
pub enum AppserviceEventKind {
    Push,
    Ping,
    Query(QueryKind),
    ThirdParty(ThirdPartyKind),
}

impl From<QueryKind> for AppserviceEventKind {
    fn from(value: QueryKind) -> Self {
        Self::Query(value)
    }
}

impl From<ThirdPartyKind> for AppserviceEventKind {
    fn from(value: ThirdPartyKind) -> Self {
        Self::ThirdParty(value)
    }
}

impl AppserviceEventKind {
    /// Check if this event matches any of a list of possible events
    pub fn matches(&self, events: Vec<AppserviceEventKind>) -> Option<AppserviceEventKind> {
        for event in events {
            #[allow(unused_parens)]
            if
                let Some(matched) = (match event.clone() {
                    AppserviceEventKind::Push | AppserviceEventKind::Ping => if self.eq(&event) {
                        Some(self.clone())
                    } else {
                        None
                    }
                    AppserviceEventKind::Query(query_kind) => if
                        let AppserviceEventKind::Query(kind) = self.clone()
                    {
                        match query_kind {
                            QueryKind::Any => Some(self.clone()),
                            other => if other == kind { Some(self.clone()) } else { None }
                        }
                    } else {
                        None
                    }
                    AppserviceEventKind::ThirdParty(third_party_kind) => if
                        let AppserviceEventKind::ThirdParty(kind) = self.clone()
                    {
                        match third_party_kind {
                            ThirdPartyKind::Any => Some(self.clone()),
                            other => if other == kind { Some(self.clone()) } else { None }
                        }
                    } else {
                        None
                    }
                })
            {
                return Some(matched);
            }
        }

        None
    }
}

#[derive(Clone, Debug)]
pub enum AppserviceEvent {
    Push(matrix_sdk::ruma::api::appservice::event::push_events::v1::Request),
    Ping(matrix_sdk::ruma::api::appservice::ping::send_ping::v1::Request),
    QueryUser(matrix_sdk::ruma::api::appservice::query::query_user_id::v1::Request),
    QueryRoomAlias(matrix_sdk::ruma::api::appservice::query::query_room_alias::v1::Request),
    ThirdPartyGetLocationForProtocol,
}
