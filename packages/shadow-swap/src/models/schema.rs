// @generated automatically by Diesel CLI.

diesel::table! {
    bridge_events (id) {
        id -> Int4,
        event_id -> Text,
        intent_id -> Nullable<Text>,
        event_type -> Text,
        event_data -> Jsonb,
        chain_id -> Int4,
        block_number -> Int8,
        transaction_hash -> Text,
        timestamp -> Timestamptz,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    chain_transactions (id) {
        id -> Int4,
        intent_id -> Text,
        chain_id -> Int4,
        tx_type -> Text,
        tx_hash -> Text,
        status -> Text,
        timestamp -> Int8,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    ethereum_sepolia_intent_created (id) {
        id -> Int4,
        event_data -> Jsonb,
        block_number -> Int8,
        log_index -> Int4,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    indexer_checkpoints (chain) {
        chain -> Text,
        last_block -> Int4,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    intent_privacy_params (intent_id) {
        intent_id -> Text,
        commitment -> Nullable<Text>,
        nullifier -> Nullable<Text>,
        secret -> Nullable<Text>,
        recipient -> Nullable<Text>,
        claim_signature -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    intents (id) {
        id -> Text,
        source_token -> Text,
        amount -> Text,
        deadline -> Int8,
        source_commitment -> Nullable<Text>,
        dest_fill_txid -> Nullable<Text>,
        status -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        #[max_length = 42]
        user_address -> Varchar,
        #[max_length = 50]
        source_chain -> Varchar,
        #[max_length = 50]
        dest_chain -> Varchar,
        #[max_length = 42]
        dest_token -> Varchar,
        #[max_length = 78]
        dest_amount -> Varchar,
        #[max_length = 66]
        source_complete_txid -> Nullable<Varchar>,
        #[max_length = 42]
        refund_address -> Nullable<Varchar>,
    }
}

diesel::table! {
    mantle_sepolia_intent_created (id) {
        id -> Int4,
        event_data -> Jsonb,
        block_number -> Int8,
        log_index -> Int4,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    merkle_nodes (node_id) {
        node_id -> Int4,
        tree_id -> Int4,
        level -> Int4,
        node_index -> Int8,
        hash -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    merkle_roots (tree_id) {
        tree_id -> Int4,
        root -> Text,
        leaf_count -> Int8,
        updated_at -> Timestamptz,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    merkle_tree_ethereum_commitments (id) {
        id -> Int4,
        commitment -> Text,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    merkle_trees (tree_id) {
        tree_id -> Int4,
        tree_name -> Text,
        depth -> Int4,
        root -> Text,
        leaf_count -> Int8,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    root_syncs (id) {
        id -> Int4,
        sync_type -> Text,
        root -> Text,
        tx_hash -> Text,
        created_at -> Timestamptz,
    }
}

diesel::joinable!(bridge_events -> intents (intent_id));
diesel::joinable!(chain_transactions -> intents (intent_id));
diesel::joinable!(intent_privacy_params -> intents (intent_id));
diesel::joinable!(merkle_nodes -> merkle_trees (tree_id));

diesel::allow_tables_to_appear_in_same_query!(
    bridge_events,
    chain_transactions,
    ethereum_sepolia_intent_created,
    indexer_checkpoints,
    intent_privacy_params,
    intents,
    mantle_sepolia_intent_created,
    merkle_nodes,
    merkle_roots,
    merkle_tree_ethereum_commitments,
    merkle_trees,
    root_syncs,
);
