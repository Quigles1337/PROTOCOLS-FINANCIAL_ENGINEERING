from pyteal import *

NUM_GLOBAL_UINTS = 2
NUM_GLOBAL_BYTES = 1
NUM_LOCAL_UINTS = 15
NUM_LOCAL_BYTES = 5

def approval_program():
    admin = Bytes("admin")
    next_channel_id = Bytes("next_ch_id")
    ch_sender = Bytes("sender")
    ch_recipient = Bytes("recipient")
    ch_token = Bytes("token")
    ch_balance = Bytes("balance")
    ch_claimed = Bytes("claimed")
    ch_nonce = Bytes("nonce")
    ch_expires_at = Bytes("expires")
    ch_status = Bytes("status")
    ch_disputed_at = Bytes("disp_at")
    ch_challenge_period = Bytes("challenge")
    
    on_creation = Seq([App.globalPut(admin, Txn.sender()), App.globalPut(next_channel_id, Int(1)), Return(Int(1))])
    
    @Subroutine(TealType.uint64)
    def create_channel():
        recipient = Txn.application_args[1]
        token_id = Btoi(Txn.application_args[2])
        amount = Btoi(Txn.application_args[3])
        duration = Btoi(Txn.application_args[4])
        challenge_period = Btoi(Txn.application_args[5])
        channel_id = App.globalGet(next_channel_id)
        sender = Txn.sender()
        return Seq([Assert(amount > Int(0)), Assert(duration > Int(0)), Assert(challenge_period > Int(0)), Assert(sender != recipient), App.localPut(sender, ch_sender, sender), App.localPut(sender, ch_recipient, recipient), App.localPut(sender, ch_token, Itob(token_id)), App.localPut(sender, ch_balance, amount), App.localPut(sender, ch_claimed, Int(0)), App.localPut(sender, ch_nonce, Int(0)), App.localPut(sender, ch_expires_at, Global.round() + duration), App.localPut(sender, ch_status, Int(0)), App.localPut(sender, ch_disputed_at, Int(0)), App.localPut(sender, ch_challenge_period, challenge_period), App.globalPut(next_channel_id, channel_id + Int(1)), Return(Int(1))])
    
    @Subroutine(TealType.uint64)
    def fund_channel():
        channel_id = Btoi(Txn.application_args[1])
        amount = Btoi(Txn.application_args[2])
        sender = Txn.sender()
        current_balance = App.localGet(sender, ch_balance)
        status = App.localGet(sender, ch_status)
        return Seq([Assert(amount > Int(0)), Assert(status == Int(0)), Assert(App.localGet(sender, ch_sender) == sender), App.localPut(sender, ch_balance, current_balance + amount), Return(Int(1))])
    
    @Subroutine(TealType.uint64)
    def extend_channel():
        channel_id = Btoi(Txn.application_args[1])
        additional_duration = Btoi(Txn.application_args[2])
        sender = Txn.sender()
        current_expires = App.localGet(sender, ch_expires_at)
        status = App.localGet(sender, ch_status)
        return Seq([Assert(additional_duration > Int(0)), Assert(status == Int(0)), Assert(App.localGet(sender, ch_sender) == sender), App.localPut(sender, ch_expires_at, current_expires + additional_duration), Return(Int(1))])
    
    @Subroutine(TealType.uint64)
    def claim_payment():
        channel_id = Btoi(Txn.application_args[1])
        claim_amount = Btoi(Txn.application_args[2])
        nonce = Btoi(Txn.application_args[3])
        signature = Txn.application_args[4]
        caller = Txn.sender()
        sender_addr = App.localGet(caller, ch_sender)
        recipient_addr = App.localGet(caller, ch_recipient)
        current_claimed = App.localGet(caller, ch_claimed)
        current_nonce = App.localGet(caller, ch_nonce)
        balance = App.localGet(caller, ch_balance)
        status = App.localGet(caller, ch_status)
        expires_at = App.localGet(caller, ch_expires_at)
        transfer_amount = claim_amount - current_claimed
        return Seq([Assert(caller == recipient_addr), Assert(status == Int(0)), Assert(Global.round() < expires_at), Assert(nonce > current_nonce), Assert(claim_amount <= balance), Assert(claim_amount > current_claimed), App.localPut(caller, ch_claimed, claim_amount), App.localPut(caller, ch_nonce, nonce), If(claim_amount >= balance).Then(App.localPut(caller, ch_status, Int(2))), Return(Int(1))])
    
    @Subroutine(TealType.uint64)
    def close_cooperative():
        channel_id = Btoi(Txn.application_args[1])
        final_amount = Btoi(Txn.application_args[2])
        sender_addr = App.localGet(Txn.sender(), ch_sender)
        recipient_addr = App.localGet(Txn.sender(), ch_recipient)
        balance = App.localGet(Txn.sender(), ch_balance)
        status = App.localGet(Txn.sender(), ch_status)
        remainder = balance - final_amount
        return Seq([Assert(status == Int(0)), Assert(final_amount <= balance), Assert(Global.group_size() >= Int(2)), App.localPut(Txn.sender(), ch_status, Int(2)), App.localPut(Txn.sender(), ch_claimed, final_amount), Return(Int(1))])
    
    @Subroutine(TealType.uint64)
    def close_unilateral():
        channel_id = Btoi(Txn.application_args[1])
        sender_addr = App.localGet(Txn.sender(), ch_sender)
        recipient_addr = App.localGet(Txn.sender(), ch_recipient)
        balance = App.localGet(Txn.sender(), ch_balance)
        claimed = App.localGet(Txn.sender(), ch_claimed)
        status = App.localGet(Txn.sender(), ch_status)
        expires_at = App.localGet(Txn.sender(), ch_expires_at)
        disputed_at = App.localGet(Txn.sender(), ch_disputed_at)
        challenge_period = App.localGet(Txn.sender(), ch_challenge_period)
        unclaimed = balance - claimed
        return Seq([Assert(Or(Txn.sender() == sender_addr, Txn.sender() == recipient_addr)), Assert(Global.round() >= expires_at), If(status == Int(1)).Then(Assert(Global.round() >= disputed_at + challenge_period)), App.localPut(Txn.sender(), ch_status, Int(2)), Return(Int(1))])
    
    @Subroutine(TealType.uint64)
    def dispute_claim():
        channel_id = Btoi(Txn.application_args[1])
        sender_addr = App.localGet(Txn.sender(), ch_sender)
        status = App.localGet(Txn.sender(), ch_status)
        return Seq([Assert(Txn.sender() == sender_addr), Assert(status == Int(0)), App.localPut(Txn.sender(), ch_status, Int(1)), App.localPut(Txn.sender(), ch_disputed_at, Global.round()), Return(Int(1))])
    
    @Subroutine(TealType.uint64)
    def get_channel():
        channel_id = Btoi(Txn.application_args[1])
        return Return(Int(1))
    
    @Subroutine(TealType.uint64)
    def get_available_balance():
        channel_id = Btoi(Txn.application_args[1])
        balance = App.localGet(Txn.sender(), ch_balance)
        claimed = App.localGet(Txn.sender(), ch_claimed)
        available = balance - claimed
        return Seq([Assert(available >= Int(0)), Return(Int(1))])
    
    program = Cond(
        [Txn.application_id() == Int(0), on_creation],
        [Txn.on_completion() == OnComplete.OptIn, Return(Int(1))],
        [Txn.on_completion() == OnComplete.CloseOut, Return(Int(1))],
        [Txn.application_args[0] == Bytes("create"), create_channel()],
        [Txn.application_args[0] == Bytes("fund"), fund_channel()],
        [Txn.application_args[0] == Bytes("extend"), extend_channel()],
        [Txn.application_args[0] == Bytes("claim"), claim_payment()],
        [Txn.application_args[0] == Bytes("coop_close"), close_cooperative()],
        [Txn.application_args[0] == Bytes("uni_close"), close_unilateral()],
        [Txn.application_args[0] == Bytes("dispute"), dispute_claim()],
        [Txn.application_args[0] == Bytes("get_channel"), get_channel()],
        [Txn.application_args[0] == Bytes("get_balance"), get_available_balance()],
    )
    return program

def clear_state_program():
    return Return(Int(1))

if __name__ == "__main__":
    print("PaymentChannels contract compiled!")
