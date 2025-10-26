"""AccountDelete-Account Lifecycle"""

from pyteal import *
NUM_GLOBAL_UINTS=3
NUM_GLOBAL_BYTES=1
NUM_LOCAL_UINTS=10
NUM_LOCAL_BYTES=5

def approval_program():
  admin=Bytes("admin")
  total_requests=Bytes("total_requests")
  total_deletions=Bytes("total_deletions")
  del_id=Bytes("id")
  del_account=Bytes("account")
  del_beneficiary=Bytes("beneficiary")
  del_requested_at=Bytes("requested")
  del_scheduled_at=Bytes("scheduled")
  del_status=Bytes("status")
  del_requires_approval=Bytes("req_approval")
  del_approvals=Bytes("approvals")
  del_asset_cleanup=Bytes("cleanup")
  on_creation=Seq([App.globalPut(admin,Txn.sender()),App.globalPut(total_requests,Int(0)),App.globalPut(total_deletions,Int(0)),Return(Int(1))])
  @Subroutine(TealType.uint64)
  def request_deletion():
    beneficiary=Txn.application_args[1]
    scheduled_at=Btoi(Txn.application_args[2])
    requires_approval=Btoi(Txn.application_args[3])
    account=Txn.sender()
    request_id=App.globalGet(total_requests)+Int(1)
    return Seq([Assert(Len(beneficiary)==Int(32)),Assert(scheduled_at>Global.round()),Assert(Or(requires_approval==Int(0),requires_approval==Int(1))),App.localPut(account,del_id,request_id),App.localPut(account,del_account,account),App.localPut(account,del_beneficiary,beneficiary),App.localPut(account,del_requested_at,Global.round()),App.localPut(account,del_scheduled_at,scheduled_at),App.localPut(account,del_status,Int(0)),App.localPut(account,del_requires_approval,requires_approval),App.localPut(account,del_approvals,Int(0)),App.localPut(account,del_asset_cleanup,Int(0)),App.globalPut(total_requests,request_id),Return(Int(1))])
  @Subroutine(TealType.uint64)
  def approve_deletion():
    request_id=Btoi(Txn.application_args[1])
    caller=Txn.sender()
    requires_approval_val=App.localGet(caller,del_requires_approval)
    status=App.localGet(caller,del_status)
    approvals=App.localGet(caller,del_approvals)
    return Seq([Assert(requires_approval_val==Int(1)),Assert(status==Int(0)),App.localPut(caller,del_approvals,approvals+Int(1)),If(approvals+Int(1)>=Int(2)).Then(App.localPut(caller,del_status,Int(1))),Return(Int(1))])
  @Subroutine(TealType.uint64)
  def mark_assets_cleaned():
    request_id=Btoi(Txn.application_args[1])
    caller=Txn.sender()
    account_addr=App.localGet(caller,del_account)
    status=App.localGet(caller,del_status)
    return Seq([Assert(caller==account_addr),Assert(Or(status==Int(0),status==Int(1))),App.localPut(caller,del_asset_cleanup,Int(1)),Return(Int(1))])
  @Subroutine(TealType.uint64)
  def execute_deletion():
    request_id=Btoi(Txn.application_args[1])
    caller=Txn.sender()
    scheduled_at=App.localGet(caller,del_scheduled_at)
    status=App.localGet(caller,del_status)
    requires_approval_val=App.localGet(caller,del_requires_approval)
    asset_cleanup_val=App.localGet(caller,del_asset_cleanup)
    return Seq([Assert(Global.round()>=scheduled_at),Assert(asset_cleanup_val==Int(1)),If(requires_approval_val==Int(1)).Then(Assert(status==Int(1))),App.localPut(caller,del_status,Int(2)),App.globalPut(total_deletions,App.globalGet(total_deletions)+Int(1)),Return(Int(1))])
  @Subroutine(TealType.uint64)
  def cancel_deletion():
    request_id=Btoi(Txn.application_args[1])
    caller=Txn.sender()
    account_addr=App.localGet(caller,del_account)
    status=App.localGet(caller,del_status)
    return Seq([Assert(caller==account_addr),Assert(Or(status==Int(0),status==Int(1))),App.localPut(caller,del_status,Int(3)),Return(Int(1))])
  @Subroutine(TealType.uint64)
  def update_beneficiary():
    request_id=Btoi(Txn.application_args[1])
    new_beneficiary=Txn.application_args[2]
    caller=Txn.sender()
    account_addr=App.localGet(caller,del_account)
    status=App.localGet(caller,del_status)
    return Seq([Assert(caller==account_addr),Assert(Or(status==Int(0),status==Int(1))),Assert(Len(new_beneficiary)==Int(32)),App.localPut(caller,del_beneficiary,new_beneficiary),Return(Int(1))])
  @Subroutine(TealType.uint64)
  def extend_schedule():
    request_id=Btoi(Txn.application_args[1])
    new_scheduled_at=Btoi(Txn.application_args[2])
    caller=Txn.sender()
    account_addr=App.localGet(caller,del_account)
    current_scheduled=App.localGet(caller,del_scheduled_at)
    status=App.localGet(caller,del_status)
    return Seq([Assert(caller==account_addr),Assert(Or(status==Int(0),status==Int(1))),Assert(new_scheduled_at>current_scheduled),App.localPut(caller,del_scheduled_at,new_scheduled_at),Return(Int(1))])
  program=Cond([Txn.application_id()==Int(0),on_creation],[Txn.on_completion()==OnComplete.OptIn,Return(Int(1))],[Txn.on_completion()==OnComplete.CloseOut,Return(Int(1))],[Txn.application_args[0]==Bytes("request"),request_deletion()],[Txn.application_args[0]==Bytes("approve"),approve_deletion()],[Txn.application_args[0]==Bytes("mark_cleanup"),mark_assets_cleaned()],[Txn.application_args[0]==Bytes("execute"),execute_deletion()],[Txn.application_args[0]==Bytes("cancel"),cancel_deletion()],[Txn.application_args[0]==Bytes("update_beneficiary"),update_beneficiary()],[Txn.application_args[0]==Bytes("extend"),extend_schedule()])
  return program
def clear_state_program():
  return Return(Int(1))
if __name__=="__main__":
  print("AccountDelete-account lifecycle!")
