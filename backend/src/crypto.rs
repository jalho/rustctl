pub fn generate_web_server_token_signing_key_pair() -> KeyPairPEM {
    let private_key_serialized_pem: PrivatePEM;
    let public_key_serialized_pem: PublicPEM;
    {
        let key_pair: aws_lc_rs::unstable::signature::PqdsaKeyPair =
            aws_lc_rs::unstable::signature::PqdsaKeyPair::generate(
                /*
                 * ML-DSA for PQC.
                 */
                &aws_lc_rs::unstable::signature::ML_DSA_87_SIGNING,
            )
            .unwrap();

        /*
         * Private key.
         */
        {
            let private_key_bin: aws_lc_rs::pkcs8::Document = key_pair.to_pkcs8().unwrap();
            let private_key_bin: &[u8] = private_key_bin.as_ref();

            private_key_serialized_pem =
                pem::encode(&pem::Pem::new("PRIVATE KEY", private_key_bin));
        }

        /*
         * Public key.
         */
        {
            let public_key: &aws_lc_rs::unstable::signature::PqdsaPublicKey =
                aws_lc_rs::signature::KeyPair::public_key(&key_pair);

            use aws_lc_rs::encoding::AsDer;
            let public_key_bin: aws_lc_rs::encoding::PublicKeyX509Der<'_> =
                public_key.as_der().unwrap();
            let public_key_bin: &[u8] = public_key_bin.as_ref();

            public_key_serialized_pem = pem::encode(&pem::Pem::new("PUBLIC KEY", public_key_bin));
        }
    }

    (private_key_serialized_pem, public_key_serialized_pem)
}

pub fn sign(private_key_serialized_pem: PrivatePEM, signable_payload: &[u8]) -> Vec<u8> {
    let signature: Vec<u8>;
    {
        let private_key_pem: pem::Pem = pem::parse(&private_key_serialized_pem).unwrap();
        let key_pair: aws_lc_rs::unstable::signature::PqdsaKeyPair =
            aws_lc_rs::unstable::signature::PqdsaKeyPair::from_pkcs8(
                /*
                 * ML-DSA for PQC.
                 */
                &aws_lc_rs::unstable::signature::ML_DSA_87_SIGNING,
                private_key_pem.contents(),
            )
            .unwrap();

        let mut signature_buf: Vec<u8> = vec![0u8; key_pair.algorithm().signature_len()];
        let _signature_len: usize = key_pair.sign(signable_payload, &mut signature_buf).unwrap();

        signature = signature_buf;
    }

    signature
}

pub fn verify(
    public_key_serialized_pem: PublicPEM,
    signature: &[u8],
    signed_payload: &[u8],
) -> Result<(), ()> {
    let public_key_pem: pem::Pem = pem::parse(&public_key_serialized_pem).unwrap();
    let public_key: aws_lc_rs::signature::UnparsedPublicKey<&[u8]> =
        aws_lc_rs::signature::UnparsedPublicKey::new(
            /*
             * ML-DSA for PQC.
             */
            &aws_lc_rs::unstable::signature::ML_DSA_87,
            public_key_pem.contents(),
        );

    match public_key.verify(signed_payload, signature) {
        Ok(_) => Ok(()),
        Err(_) => Err(()),
    }
}

pub type KeyPairPEM = (PrivatePEM, PublicPEM);
pub type PrivatePEM = String;
pub type PublicPEM = String;
