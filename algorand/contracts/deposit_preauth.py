"""
DepositPreauth - One-Time Pre-Authorized Deposits
Production-grade Algorand PyTeal implementation
"""

from pyteal import *

NUM_GLOBAL_UINTS = 3
NUM_GLOBAL_BYTES = 1
NUM_LOCAL_UINTS = 8
NUM_LOCAL_BYTES = 4

def approval_program():
    admin = Bytes("admin")
    total_preauths = Bytes("total_preauths")
    used_preauths = Bytes("used_preauths")

    preauth_id = Bytes("id")
    preauth_authorizer = Bytes("authorizer")
    preauth_authorized = Bytes("authorized")
    preauth_asset = Bytes("asset")
    preauth_created_at = Bytes("created")
    preauth_expires_at = Bytes("expires")
    preauth_status = Bytes("status")
    preauth_used_at = Bytes("used_at")

    on_creation = Seq([
        App.globalPut(admin, Txn.sender()),
        App.globalPut(total_preauths, Int(0)),
        App.globalPut(used_preauths, Int(0)),
        Return(Int(1))
    ])

    @Subroutine(TealType.uint64)
    def create_preauth():
        authorized_addr = Txn.application_args[1]
        asset_id = Btoi(Txn.application_args[2])
        expires_at = Btoi(Txn.application_args[3])
        authorizer = Txn.sender()

        return Seq([
            Assert(Len(authorized_addr) == Int(32)),
            Assert(authorizer != authorized_addr),
            If(expires_at > Int(0)).Then(Assert(expires_at > Global.round())),
            App.localPut(authorizer, preauth_authorizer, authorizer),
            App.localPut(authorizer, preauth_authorized, authorized_addr),
            App.localPut(authorizer, preauth_asset, Itob(asset_id)),
            App.localPut(authorizer, preauth_created_at, Global.round()),
            App.localPut(authorizer, preauth_expires_at, expires_at),
            App.localPut(authorizer, preauth_status, Int(0)),
            App.localPut(authorizer, preauth_used_at, Int(0)),
            App.globalPut(total_preauths, App.globalGet(total_preauths) + Int(1)),
            Return(Int(1))
        ])

    @Subroutine(TealType.uint64)
    def use_preauth():
        preauth_id_arg = Btoi(Txn.application_args[1])
        caller = Txn.sender()
        authorized_addr = App.localGet(caller, preauth_authorized)
        status = App.localGet(caller, preauth_status)
        expires_at_val = App.localGet(caller, preauth_expires_at)

        return Seq([
            Assert(caller == authorized_addr),
            Assert(status == Int(0)),
            If(expires_at_val > Int(0)).Then(Assert(Global.round() < expires_at_val)),
            App.localPut(caller, preauth_status, Int(1)),
            App.localPut(caller, preauth_used_at, Global.round()),
            App.globalPut(used_preauths, App.globalGet(used_preauths) + Int(1)),
            Return(Int(1))
        ])

    @Subroutine(TealType.uint64)
    def revoke_preauth():
        preauth_id_arg = Btoi(Txn.application_args[1])
        caller = Txn.sender()
        authorizer_addr = App.localGet(caller, preauth_authorizer)
        status = App.localGet(caller, preauth_status)

        return Seq([
            Assert(caller == authorizer_addr),
            Assert(status == Int(0)),
            App.localPut(caller, preauth_status, Int(2)),
            Return(Int(1))
        ])

    @Subroutine(TealType.uint64)
    def mark_expired():
        preauth_id_arg = Btoi(Txn.application_args[1])
        caller = Txn.sender()
        status = App.localGet(caller, preauth_status)
        expires_at_val = App.localGet(caller, preauth_expires_at)

        return Seq([
            Assert(status == Int(0)),
            Assert(expires_at_val > Int(0)),
            Assert(Global.round() >= expires_at_val),
            App.localPut(caller, preauth_status, Int(3)),
            Return(Int(1))
        ])

    program = Cond(
        [Txn.application_id() == Int(0), on_creation],
        [Txn.on_completion() == OnComplete.OptIn, Return(Int(1))],
        [Txn.on_completion() == OnComplete.CloseOut, Return(Int(1))],
        [Txn.application_args[0] == Bytes("create"), create_preauth()],
        [Txn.application_args[0] == Bytes("use"), use_preauth()],
        [Txn.application_args[0] == Bytes("revoke"), revoke_preauth()],
        [Txn.application_args[0] == Bytes("mark_expired"), mark_expired()],
    )
    return program

def clear_state_program():
    return Return(Int(1))

if __name__ == "__main__":
    print("DepositPreauth contract compiled - one-time authorizations!")
