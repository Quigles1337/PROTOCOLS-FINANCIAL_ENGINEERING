#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod deposit_authorization {
    use ink::storage::Mapping;

    #[ink(storage)]
    pub struct DepositAuthorization {
        admin: AccountId,
        compliance_officer: AccountId,
        authorizations: Mapping<(AccountId, AccountId, AssetId), Authorization>,
        global_authorizations: Mapping<AccountId, GlobalAuth>,
        auth_counter: u64,
    }

    pub type AssetId = u32;

    #[derive(scale::Decode, scale::Encode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, Debug, PartialEq, Clone))]
    pub struct Authorization {
        pub authorizer: AccountId,
        pub authorized: AccountId,
        pub asset_id: AssetId,
        pub max_amount: Balance,
        pub expiration: u64,
        pub tier: AuthTier,
        pub status: AuthStatus,
        pub created_at: u64,
    }

    #[derive(scale::Decode, scale::Encode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, Debug, PartialEq, Clone))]
    pub struct GlobalAuth {
        pub kyc_level: u8,
        pub aml_verified: bool,
        pub jurisdictions: u32,
        pub blocked: bool,
    }

    #[derive(scale::Decode, scale::Encode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, Debug, PartialEq, Clone))]
    pub enum AuthTier {
        Basic,
        Standard,
        Premium,
        Institutional,
    }

    #[derive(scale::Decode, scale::Encode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, Debug, PartialEq, Clone))]
    pub enum AuthStatus {
        Active,
        Suspended,
        Revoked,
    }

    #[ink(event)]
    pub struct AuthorizationCreated {
        #[ink(topic)]
        authorizer: AccountId,
        authorized: AccountId,
        asset_id: AssetId,
        max_amount: Balance,
        tier: AuthTier,
    }

    #[ink(event)]
    pub struct AuthorizationRevoked {
        #[ink(topic)]
        authorizer: AccountId,
        authorized: AccountId,
        asset_id: AssetId,
    }

    #[ink(event)]
    pub struct GlobalAuthUpdated {
        #[ink(topic)]
        account: AccountId,
        kyc_level: u8,
    }

    impl DepositAuthorization {
        #[ink(constructor)]
        pub fn new(compliance_officer: AccountId) -> Self {
            Self {
                admin: Self::env().caller(),
                compliance_officer,
                authorizations: Mapping::new(),
                global_authorizations: Mapping::new(),
                auth_counter: 0,
            }
        }

        #[ink(message)]
        pub fn create_authorization(
            &mut self,
            authorized: AccountId,
            asset_id: AssetId,
            max_amount: Balance,
            expiration: u64,
            tier: AuthTier,
        ) {
            let authorizer = self.env().caller();
            let current_block = self.env().block_number();

            assert!(
                authorizer == self.admin || authorizer == self.compliance_officer,
                "Unauthorized"
            );
            assert!(expiration > current_block, "Expiration must be in future");

            let auth = Authorization {
                authorizer,
                authorized,
                asset_id,
                max_amount,
                expiration,
                tier: tier.clone(),
                status: AuthStatus::Active,
                created_at: current_block,
            };

            self.authorizations.insert((authorizer, authorized, asset_id), &auth);
            self.auth_counter += 1;

            self.env().emit_event(AuthorizationCreated {
                authorizer,
                authorized,
                asset_id,
                max_amount,
                tier,
            });
        }

        #[ink(message)]
        pub fn revoke_authorization(
            &mut self,
            authorized: AccountId,
            asset_id: AssetId,
        ) {
            let authorizer = self.env().caller();
            let key = (authorizer, authorized, asset_id);
            let mut auth = self.authorizations.get(key).expect("Authorization not found");

            assert!(
                authorizer == auth.authorizer || authorizer == self.admin,
                "Unauthorized"
            );

            auth.status = AuthStatus::Revoked;
            self.authorizations.insert(key, &auth);

            self.env().emit_event(AuthorizationRevoked {
                authorizer,
                authorized,
                asset_id,
            });
        }

        #[ink(message)]
        pub fn set_global_auth(
            &mut self,
            account: AccountId,
            kyc_level: u8,
            aml_verified: bool,
            jurisdictions: u32,
        ) {
            let caller = self.env().caller();
            assert!(caller == self.compliance_officer, "Only compliance officer");

            let global_auth = GlobalAuth {
                kyc_level,
                aml_verified,
                jurisdictions,
                blocked: false,
            };

            self.global_authorizations.insert(account, &global_auth);

            self.env().emit_event(GlobalAuthUpdated {
                account,
                kyc_level,
            });
        }

        #[ink(message)]
        pub fn block_account(&mut self, account: AccountId) {
            let caller = self.env().caller();
            assert!(
                caller == self.admin || caller == self.compliance_officer,
                "Unauthorized"
            );

            let mut global_auth = self.global_authorizations
                .get(account)
                .unwrap_or(GlobalAuth {
                    kyc_level: 0,
                    aml_verified: false,
                    jurisdictions: 0,
                    blocked: false,
                });

            global_auth.blocked = true;
            self.global_authorizations.insert(account, &global_auth);
        }

        #[ink(message)]
        pub fn check_authorization(
            &self,
            authorizer: AccountId,
            authorized: AccountId,
            asset_id: AssetId,
            amount: Balance,
        ) -> bool {
            let global_auth = self.global_authorizations.get(authorized);
            if let Some(ga) = global_auth {
                if ga.blocked || !ga.aml_verified {
                    return false;
                }
            }

            if let Some(auth) = self.authorizations.get((authorizer, authorized, asset_id)) {
                matches!(auth.status, AuthStatus::Active)
                    && self.env().block_number() < auth.expiration
                    && amount <= auth.max_amount
            } else {
                false
            }
        }

        #[ink(message)]
        pub fn get_authorization(
            &self,
            authorizer: AccountId,
            authorized: AccountId,
            asset_id: AssetId,
        ) -> Option<Authorization> {
            self.authorizations.get((authorizer, authorized, asset_id))
        }

        #[ink(message)]
        pub fn get_global_auth(&self, account: AccountId) -> Option<GlobalAuth> {
            self.global_authorizations.get(account)
        }

        #[ink(message)]
        pub fn get_auth_count(&self) -> u64 {
            self.auth_counter
        }
    }
}
