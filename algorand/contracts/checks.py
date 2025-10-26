from pyteal import *

NUM_GLOBAL_UINTS = 3
NUM_GLOBAL_BYTES = 1
NUM_LOCAL_UINTS = 10
NUM_LOCAL_BYTES = 6

def approval_program():
    admin = Bytes("admin")
    next_check_id = Bytes("next_chk_id")
    total_checks = Bytes("total_chks")
    chk_id = Bytes("id")
    chk_issuer = Bytes("issuer")
    chk_payee = Bytes("payee")
    chk_token = Bytes("token")
    chk_amount = Bytes("amount")
    chk_cashed_amount = Bytes("cashed")
    chk_check_type = Bytes("type")
    chk_expires_at = Bytes("expires")
    chk_status = Bytes("status")
    chk_created_at = Bytes("created")
    chk_memo = Bytes("memo")
    chk_allow_partial = Bytes("partial")
    
    on_creation = Seq([App.globalPut(admin, Txn.sender()), App.globalPut(next_check_id, Int(1)), App.globalPut(total_checks, Int(0)), Return(Int(1))])
    
    @Subroutine(TealType.uint64)
    def create_check():
        payee = Txn.application_args[1]
        token_id = Btoi(Txn.application_args[2])
        amount = Btoi(Txn.application_args[3])
        check_type = Btoi(Txn.application_args[4])
        expires_at = Btoi(Txn.application_args[5])
        allow_partial = Btoi(Txn.application_args[6])
        memo = Txn.application_args[7] if Len(Txn.application_args) > Int(7) else Bytes("")
        check_id = App.globalGet(next_check_id)
        issuer = Txn.sender()
        return Seq([Assert(amount > Int(0)), Assert(Or(check_type == Int(0), check_type == Int(1))), Assert(Or(allow_partial == Int(0), allow_partial == Int(1))), If(check_type == Int(1)).Then(Assert(Len(payee) == Int(32))), If(expires_at > Int(0)).Then(Assert(expires_at > Global.round())), App.localPut(issuer, chk_id, check_id), App.localPut(issuer, chk_issuer, issuer), App.localPut(issuer, chk_payee, payee), App.localPut(issuer, chk_token, Itob(token_id)), App.localPut(issuer, chk_amount, amount), App.localPut(issuer, chk_cashed_amount, Int(0)), App.localPut(issuer, chk_check_type, check_type), App.localPut(issuer, chk_expires_at, expires_at), App.localPut(issuer, chk_status, Int(0)), App.localPut(issuer, chk_created_at, Global.round()), App.localPut(issuer, chk_memo, memo), App.localPut(issuer, chk_allow_partial, allow_partial), App.globalPut(next_check_id, check_id + Int(1)), App.globalPut(total_checks, App.globalGet(total_checks) + Int(1)), Return(Int(1))])
    
    @Subroutine(TealType.uint64)
    def cash_check():
        check_id_arg = Btoi(Txn.application_args[1])
        cash_amount_arg = Btoi(Txn.application_args[2]) if Len(Txn.application_args) > Int(2) else Int(0)
        caller = Txn.sender()
        issuer_addr = App.localGet(caller, chk_issuer)
        payee_addr = App.localGet(caller, chk_payee)
        check_type_val = App.localGet(caller, chk_check_type)
        amount = App.localGet(caller, chk_amount)
        cashed = App.localGet(caller, chk_cashed_amount)
        status = App.localGet(caller, chk_status)
        expires_at_val = App.localGet(caller, chk_expires_at)
        allow_partial_val = App.localGet(caller, chk_allow_partial)
        remaining = amount - cashed
        actual_cash_amount = If(cash_amount_arg == Int(0), remaining, cash_amount_arg)
        new_cashed = cashed + actual_cash_amount
        return Seq([Assert(status == Int(0)), If(expires_at_val > Int(0)).Then(Assert(Global.round() < expires_at_val)), If(check_type_val == Int(1)).Then(Assert(caller == payee_addr)), Assert(actual_cash_amount > Int(0)), Assert(actual_cash_amount <= remaining), If(allow_partial_val == Int(0)).Then(Assert(actual_cash_amount == remaining)), App.localPut(caller, chk_cashed_amount, new_cashed), If(new_cashed >= amount).Then(App.localPut(caller, chk_status, Int(1))), Return(Int(1))])
    
    @Subroutine(TealType.uint64)
    def cancel_check():
        check_id_arg = Btoi(Txn.application_args[1])
        caller = Txn.sender()
        issuer_addr = App.localGet(caller, chk_issuer)
        status = App.localGet(caller, chk_status)
        cashed = App.localGet(caller, chk_cashed_amount)
        return Seq([Assert(caller == issuer_addr), Assert(status == Int(0)), Assert(cashed == Int(0)), App.localPut(caller, chk_status, Int(2)), Return(Int(1))])
    
    @Subroutine(TealType.uint64)
    def extend_expiration():
        check_id_arg = Btoi(Txn.application_args[1])
        new_expires_at = Btoi(Txn.application_args[2])
        caller = Txn.sender()
        issuer_addr = App.localGet(caller, chk_issuer)
        status = App.localGet(caller, chk_status)
        current_expires = App.localGet(caller, chk_expires_at)
        return Seq([Assert(caller == issuer_addr), Assert(status == Int(0)), Assert(new_expires_at > current_expires), Assert(new_expires_at > Global.round()), App.localPut(caller, chk_expires_at, new_expires_at), Return(Int(1))])
    
    @Subroutine(TealType.uint64)
    def mark_expired():
        check_id_arg = Btoi(Txn.application_args[1])
        caller = Txn.sender()
        status = App.localGet(caller, chk_status)
        expires_at_val = App.localGet(caller, chk_expires_at)
        return Seq([Assert(status == Int(0)), Assert(expires_at_val > Int(0)), Assert(Global.round() >= expires_at_val), App.localPut(caller, chk_status, Int(3)), Return(Int(1))])
    
    @Subroutine(TealType.uint64)
    def get_check_info():
        check_id_arg = Btoi(Txn.application_args[1])
        return Return(Int(1))
    
    @Subroutine(TealType.uint64)
    def get_remaining_amount():
        check_id_arg = Btoi(Txn.application_args[1])
        caller = Txn.sender()
        amount = App.localGet(caller, chk_amount)
        cashed = App.localGet(caller, chk_cashed_amount)
        remaining = amount - cashed
        return Seq([Assert(remaining >= Int(0)), Return(Int(1))])
    
    @Subroutine(TealType.uint64)
    def transfer_check():
        check_id_arg = Btoi(Txn.application_args[1])
        new_payee = Txn.application_args[2]
        caller = Txn.sender()
        check_type_val = App.localGet(caller, chk_check_type)
        status = App.localGet(caller, chk_status)
        return Seq([Assert(check_type_val == Int(0)), Assert(status == Int(0)), Assert(Len(new_payee) == Int(32)), App.localPut(caller, chk_payee, new_payee), Return(Int(1))])
    
    program = Cond([Txn.application_id() == Int(0), on_creation], [Txn.on_completion() == OnComplete.OptIn, Return(Int(1))], [Txn.on_completion() == OnComplete.CloseOut, Return(Int(1))], [Txn.application_args[0] == Bytes("create"), create_check()], [Txn.application_args[0] == Bytes("cash"), cash_check()], [Txn.application_args[0] == Bytes("cancel"), cancel_check()], [Txn.application_args[0] == Bytes("extend"), extend_expiration()], [Txn.application_args[0] == Bytes("mark_expired"), mark_expired()], [Txn.application_args[0] == Bytes("get_info"), get_check_info()], [Txn.application_args[0] == Bytes("get_remaining"), get_remaining_amount()], [Txn.application_args[0] == Bytes("transfer"), transfer_check()])
    return program

def clear_state_program():
    return Return(Int(1))

if __name__ == "__main__":
    print("Checks contract")
