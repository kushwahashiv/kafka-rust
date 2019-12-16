// table! {
//       transaction (id) {
//         id -> Text,
//         account_no -> Text,
//         amount -> Double,
//         new_balance -> Double,
//         account_type -> Text,
//         changed_by -> Text,
//         from_to -> Text,
//         description -> Text,
//         created_at -> Timestamp
//     }
// }

table! {
    balance (balance_id) {
        balance_id -> Int4,
        iban -> Text,
        token -> Text,
        amount -> Int8,
        type_ -> Text,
        lmt -> Int8,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

table! {
    cacr (uuid) {
        uuid -> Text,
        iban -> Nullable<Text>,
        token -> Nullable<Text>,
        type_ -> Nullable<Text>,
        reason -> Nullable<Text>,
        created_at -> Timestamp,
    }
}

table! {
    cmtr (uuid) {
        uuid -> Text,
        reason -> Nullable<Text>,
        created_at -> Timestamp,
    }
}

allow_tables_to_appear_in_same_query!(balance, cacr, cmtr,);
