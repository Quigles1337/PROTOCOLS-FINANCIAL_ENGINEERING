from pyteal import *

NUM_GLOBAL_UINTS = 2
NUM_GLOBAL_BYTES = 1
NUM_LOCAL_UINTS = 12
NUM_LOCAL_BYTES = 8

def approval_program():
    admin = Bytes("admin")
    next_escrow_id = Bytes("next_esc_id")
    
    esc_sender = Bytes("sender")
    esc_recipient = Bytes("recipient")
    esc_token = Bytes("token")
    esc_amount = Bytes("amount")
    esc_condition_type = Bytes("cond_type")
    esc_hash_lock = Bytes("hash_lock")
    esc_time_lock = Bytes("time_lock")
    esc_expires_at = Bytes("expires")
    esc_status = Bytes("status")
    esc_allow_clawback = Bytes("clawback")
    esc_created_at = Bytes("created")
    esc_finished_at = Bytes("finished")
    
    on_creation = Seq([App.globalPut(admin, Txn.sender()), App.globalPut(next_escrow_id, Int(1)), Return(Int(1))])
    
    @Subroutine(TealType.uint64)
    def create_escrow():
        recipient = Txn.application_args[1]
        token_id = Btoi(Txn.application_args[2])
        amount = Btoi(Txn.application_args[3])
        condition_type = Btoi(Txn.application_args[4])
        hash_lock = Txn.application_args[5] if Len(Txn.application_args) > Int(5) else Bytes("")
        time_lock = Btoi(Txn.application_args[6]) if Len(Txn.application_args) > Int(6) else Int(0)
        expires_at = Btoi(Txn.application_args[7])
        allow_clawback = Btoi(Txn.application_args[8])
        
        escrow_id = App.globalGet(next_escrow_id)
        sender = Txn.sender()
        
        return Seq([Assert(amount > Int(0)), Assert(expires_at > Global.round()), Assert(sender != recipient), Assert(Or(condition_type == Int(0), condition_type == Int(1), condition_type == Int(2), condition_type == Int(3))), App.localPut(sender, esc_sender, sender), App.localPut(sender, esc_recipient, recipient), App.localPut(sender, esc_token, Itob(token_id)), App.localPut(sender, esc_amount, amount), App.localPut(sender, esc_condition_type, condition_type), App.localPut(sender, esc_hash_lock, hash_lock), App.localPut(sender, esc_time_lock, time_lock), App.localPut(sender, esc_expires_at, expires_at), App.localPut(sender, esc_status, Int(0)), App.localPut(sender, esc_allow_clawback, allow_clawback), App.localPut(sender, esc_created_at, Global.round()), App.localPut(sender, esc_finished_at, Int(0)), App.globalPut(next_escrow_id, escrow_id + Int(1)), Return(Int(1))])
    
    @Subroutine(TealType.uint64)
    def execute_escrow():
        escrow_id = Btoi(Txn.application_args[1])
        preimage = Txn.application_args[2] if Len(Txn.application_args) > Int(2) else Bytes("")
        
        caller = Txn.sender()
        sender_addr = App.localGet(caller, esc_sender)
        recipient_addr = App.localGet(caller, esc_recipient)
        condition_type = App.localGet(caller, esc_condition_type)
        hash_lock = App.localGet(caller, esc_hash_lock)
        time_lock = App.localGet(caller, esc_time_lock)
        status = App.localGet(caller, esc_status)
        expires_at = App.localGet(caller, esc_expires_at)
        
        return Seq([Assert(caller == recipient_addr), Assert(status == Int(0)), Assert(Global.round() < expires_at), If(condition_type == Int(1)).Then(Seq([Assert(Len(preimage) == Int(32)), Assert(Sha256(preimage) == hash_lock)])), If(condition_type == Int(2)).Then(Assert(Global.round() >= time_lock)), If(condition_type == Int(3)).Then(Seq([Assert(Len(preimage) == Int(32)), Assert(Sha256(preimage) == hash_lock), Assert(Global.round() >= time_lock)])), App.localPut(caller, esc_status, Int(1)), App.localPut(caller, esc_finished_at, Global.round()), Return(Int(1))])
    
    @Subroutine(TealType.uint64)
    def cancel_escrow():
        escrow_id = Btoi(Txn.application_args[1])
        caller = Txn.sender()
        sender_addr = App.localGet(caller, esc_sender)
        status = App.localGet(caller, esc_status)
        expires_at = App.localGet(caller, esc_expires_at)
        
        return Seq([Assert(caller == sender_addr), Assert(status == Int(0)), Assert(Global.round() >= expires_at), App.localPut(caller, esc_status, Int(2)), App.localPut(caller, esc_finished_at, Global.round()), Return(Int(1))])
    
    @Subroutine(TealType.uint64)
    def clawback_escrow():
        escrow_id = Btoi(Txn.application_args[1])
        caller = Txn.sender()
        sender_addr = App.localGet(caller, esc_sender)
        status = App.localGet(caller, esc_status)
        allow_clawback = App.localGet(caller, esc_allow_clawback)
        
        return Seq([Assert(caller == sender_addr), Assert(status == Int(0)), Assert(allow_clawback == Int(1)), App.localPut(caller, esc_status, Int(3)), App.localPut(caller, esc_finished_at, Global.round()), Return(Int(1))])
    
    @Subroutine(TealType.uint64)
    def get_escrow():
        escrow_id = Btoi(Txn.application_args[1])
        return Return(Int(1))
    
    program = Cond(
        [Txn.application_id() == Int(0), on_creation],
        [Txn.on_completion() == OnComplete.OptIn, Return(Int(1))],
        [Txn.on_completion() == OnComplete.CloseOut, Return(Int(1))],
        [Txn.application_args[0] == Bytes("create"), create_escrow()],
        [Txn.application_args[0] == Bytes("execute"), execute_escrow()],
        [Txn.application_args[0] == Bytes("cancel"), cancel_escrow()],
        [Txn.application_args[0] == Bytes("clawback"), clawback_escrow()],
        [Txn.application_args[0] == Bytes("get_escrow"), get_escrow()],
    )
    return program

def clear_state_program():
    return Return(Int(1))

if __name__ == "__main__":
    print("Escrow HTLC contract compiled!")
