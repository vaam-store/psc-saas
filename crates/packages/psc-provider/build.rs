extern crate tonic_build;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_prost_build::configure()
        .build_server(false) // We only need client types for now
        .compile_protos(
            &[
                "../../../protos/psc/common/v1/common.proto",
                "../../../protos/psc/payment/v1/payment.proto",
                "../../../protos/psc/payout/v1/payout.proto",
                "../../../protos/psc/journal/v1/journal.proto",
                "../../../protos/psc/balance/v1/balance.proto",
            ],
            &["../../../protos"], // Specify the root directory for proto imports
        )?;
    Ok(())
}
