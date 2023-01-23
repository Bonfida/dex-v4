import { Connection, DataSizeFilter, GetProgramAccountsFilter, MemcmpFilter } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID, AccountLayout } from "@solana/spl-token";
import fs from "fs";

const connection = new Connection(
    "https://weathered-muddy-river.solana-mainnet.quiknode.pro/ac8c6a1c4795e3477693b0afb509461af42f8d22/"
);

const getFilter = (mint: string) => {
    const filters: MemcmpFilter = {
        memcmp: { offset: 0, bytes: mint },
    };
    return filters;
};

const mints = [
    "64SqEfHtu4bZ6jr1mAxaWrLFdMngbKbru9AyaG2Dyk5T",
    "9axWWN2FY8njSSQReepkiSE56U2yAvPFGuaXRQNdkZaS",
];

interface Account {
    owner: string;
    amount: string;
}

const dev = async () => {
    console.log("Starting");
    for (let mint of mints) {
        const filters: GetProgramAccountsFilter[] = [getFilter(mint), { dataSize: 165 }];
        const result = await connection.getProgramAccounts(TOKEN_PROGRAM_ID, {
            filters,
            dataSlice: {
                length: 165,
                offset: 0,
            },
        });
        console.log(result);
        const accounts: Account[] = [];
        for (let acc of result) {
            const des = AccountLayout.decode(acc.account.data);
            accounts.push({
                owner: des.owner.toBase58(),
                amount: des.amount.toString(),
            });
        }
        fs.writeFileSync(mint + ".json", JSON.stringify(result));
    }
};

dev();