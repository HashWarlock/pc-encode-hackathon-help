import { ContractType } from '@devphase/service';
import * as PhalaSdk from '@phala/sdk';
import type { KeyringPair } from '@polkadot/keyring/types';
import { stringToHex } from '@polkadot/util';
import { OhMyChess } from "@/typings/OhMyChess";

describe("OhMyChess test", () => {
    let factory: OhMyChess.Factory;
    let contract: OhMyChess.Contract;
    let signer: KeyringPair;
    let certificate : PhalaSdk.CertificateData;

    before(async function setup(): Promise<void> {
        factory = await this.devPhase.getFactory(
            './contracts/oh_my_chess/target/ink/oh_my_chess.contract',
            { contractType: ContractType.InkCode }
        );

        await factory.deploy();

        signer = this.devPhase.accounts.bob;
        certificate = await PhalaSdk.signCertificate({
            api: this.api,
            pair: signer,
        });
    });

    describe('default constructor', () => {
        before(async function() {
            contract = await factory.instantiate('default', []);
        });

        it('Should be able...', async function() {
            const response = await contract.query.get(certificate, {});
            console.log(response.output.toJSON());
        });
    });
});
