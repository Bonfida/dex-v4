export enum Side {
  Bid = 0,
  Ask = 1,
}

export enum OrderType {
  Limit = 0,
  ImmediateOrCancel = 1,
  FillOrKill = 2,
  PostOnly = 3,
}

export enum SelfTradeBehavior {
  DecrementTake = 0,
  CancelProvide = 1,
  AbortTransaction = 2,
}
