export const FEES = {
  6: {
    fees: { taker: 0.001, maker: -0.0005 },
    srm: Infinity,
    msrm: 1,
  },
  5: {
    fees: { taker: 0.0012, maker: -0.0003 },
    srm: 1_000_000,
    msrm: Infinity,
  },
  4: {
    fees: { taker: 0.0014, maker: -0.0003 },
    srm: 100_000,
    msrm: Infinity,
  },
  3: {
    fees: { taker: 0.0016, maker: -0.0003 },
    srm: 10_000,
    msrm: Infinity,
  },
  2: {
    fees: { taker: 0.0018, maker: -0.0003 },
    srm: 1_000,
    msrm: Infinity,
  },
  1: {
    fees: { taker: 0.002, maker: -0.0003 },
    srm: 100,
    msrm: Infinity,
  },
  0: {
    fees: { taker: 0.0022, maker: -0.0003 },
    srm: 0,
    msrm: Infinity,
  },
};

export const getFeeRates = (feeTier: number) => {
  return FEES[feeTier].fees;
};

export const getFeeTier = (msrmBalance: number, srmBalance: number) => {
  if (msrmBalance >= 1) {
    return 6;
  }
  for (let [key, value] of Object.entries(FEES)) {
    if (srmBalance < value.srm) return parseInt(key) - 1;
  }
};
