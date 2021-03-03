initSidebarItems({"attr":[["async_trait",""]],"derive":[["Outgoing","Derive the `Outgoing` trait, possibly generating an ‘Incoming’ version of the struct this derive macro is used on. Specifically, if no lifetime variables are used on any of the fields of the struct, this simple implementation will be generated:"]],"enum":[["AuthScheme","Authentication scheme used by the endpoint."],["CanonicalJsonValue",""],["CustomEvent","This represents the various “unrecognized” events."],["Error","Internal representation of errors."],["FromHttpRequestError","An error when converting a http request to one of ruma’s endpoint-specific request types."],["FromHttpResponseError","An error when converting a http response to one of Ruma’s endpoint-specific response types."],["IntoHttpError","An error when converting one of ruma’s endpoint-specific request or response types to the corresponding http type."],["RoomState","An enum that represents the state of the given `Room`."],["RoomType","Enum keeping track in which state the room is, e.g. if our own user is joined, invited, or has left the room."],["ServerError","An error was reported by the server (HTTP status code 4xx or 5xx)"],["StoreError","State store specific error type."]],"macro":[["assign","Mutate a struct value in a declarative style."],["int","Creates an `Int` from a numeric literal."],["uint","Creates a `UInt` from a numeric literal."]],"mod":[["api","Crate ruma_client_api contains serializable types for the requests and responses for each endpoint in the Matrix client API specification. These types can be shared by client and server code."],["deserialized_responses",""],["directory","Common types for room directory endpoints"],["encryption","Common types for encryption related tasks."],["events","Crate `ruma_events` contains serializable types for the events in the Matrix specification that can be shared by client and server code."],["instant",""],["locks",""],["presence","Common types for the presence module"],["push","Common types for the push notifications module"],["thirdparty","Common types for the third party networks module"],["uuid","Generate and parse UUIDs."]],"struct":[["BaseClient","A no IO Client implementation."],["BaseClientConfig","Configuration for the creation of the `BaseClient`."],["Int","An integer limited to the range of integers that can be represented exactly by an f64."],["InvitedRoom","A room in an invited state."],["JoinedRoom","A room in a joined state."],["LeftRoom","A room in a left state."],["Raw","A wrapper around `Box<RawValue>`, to be used in place of any type in the Matrix endpoint definition to allow request and response types to contain that said type represented by the generic argument `Ev`."],["Room","The underlying room data structure collecting state for joined and left rooms."],["RoomInfo","The underlying pure data structure for joined and left rooms."],["RoomMember","A member of a room."],["Session","A user session, containing an access token and information about the associated user account."],["StateChanges","Store state changes and pass them to the StateStore."],["Store","A state store wrapper for the SDK."],["StrippedRoom","The underlying room data structure collecting state for invited rooms."],["StrippedRoomInfo","The underlying pure data structure for invited rooms."],["UInt","An integer limited to the range of non-negative integers that can be represented exactly by an f64."]],"trait":[["AsyncTraitDeps","Super trait that is used for our store traits, this trait will differ if it’s used on WASM. WASM targets will not require `Send` and `Sync` to have implemented, while other targets will."],["EndpointError","Gives users the ability to define their own serializable / deserializable errors."],["EventHandler","This trait allows any type implementing `EventHandler` to specify event callbacks for each event. The `Client` calls each method when the corresponding event is received."],["Outgoing","A type that can be sent to another party that understands the matrix protocol. If any of the fields of `Self` don’t implement serde’s `Deserialize`, you can derive this trait to generate a corresponding ‘Incoming’ type that supports deserialization. This is useful for things like ruma_events’ `EventResult` type. For more details, see the derive macro’s documentation."],["OutgoingRequest","A request type for a Matrix API endpoint. (trait used for sending requests)"],["StateStore","An abstract state store trait that can be used to implement different stores for the SDK."]],"type":[["Result","Result type of the rust-sdk."]]});