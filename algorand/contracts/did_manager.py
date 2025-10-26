"""DIDManager-W3C DID"""

from pyteal import *
NUM_GLOBAL_UINTS=3
NUM_GLOBAL_BYTES=1
NUM_LOCAL_UINTS=8
NUM_LOCAL_BYTES=10

def approval_program():
  admin=Bytes("admin")
  total_dids=Bytes("total_dids")
  active_dids=Bytes("active_dids")
  did_controller=Bytes("controller")
  did_status=Bytes("status")
  did_version=Bytes("version")
  on_creation=Seq([App.globalPut(admin,Txn.sender()),App.globalPut(total_dids,Int(0)),App.globalPut(active_dids,Int(0)),Return(Int(1))])
  @Subroutine(TealType.uint64)
  def create_did():
    controller=Txn.application_args[1]
    subject=Txn.sender()
    return Seq([Assert(Len(controller)==Int(32)),App.localPut(subject,did_controller,controller),App.localPut(subject,did_status,Int(0)),App.localPut(subject,did_version,Int(1)),App.globalPut(total_dids,App.globalGet(total_dids)+Int(1)),App.globalPut(active_dids,App.globalGet(active_dids)+Int(1)),Return(Int(1))])
  @Subroutine(TealType.uint64)
  def deactivate_did():
    caller=Txn.sender()
    status=App.localGet(caller,did_status)
    return Seq([Assert(status==Int(0)),App.localPut(caller,did_status,Int(1)),App.globalPut(active_dids,App.globalGet(active_dids)-Int(1)),Return(Int(1))])
  @Subroutine(TealType.uint64)
  def reactivate_did():
    caller=Txn.sender()
    status=App.localGet(caller,did_status)
    return Seq([Assert(status==Int(1)),App.localPut(caller,did_status,Int(0)),App.globalPut(active_dids,App.globalGet(active_dids)+Int(1)),Return(Int(1))])
  @Subroutine(TealType.uint64)
  def revoke_did():
    caller=Txn.sender()
    status=App.localGet(caller,did_status)
    return Seq([Assert(status\!=Int(2)),App.localPut(caller,did_status,Int(2)),If(status==Int(0)).Then(App.globalPut(active_dids,App.globalGet(active_dids)-Int(1))),Return(Int(1))])
  program=Cond([Txn.application_id()==Int(0),on_creation],[Txn.on_completion()==OnComplete.OptIn,Return(Int(1))],[Txn.on_completion()==OnComplete.CloseOut,Return(Int(1))],[Txn.application_args[0]==Bytes("create"),create_did()],[Txn.application_args[0]==Bytes("deactivate"),deactivate_did()],[Txn.application_args[0]==Bytes("reactivate"),reactivate_did()],[Txn.application_args[0]==Bytes("revoke"),revoke_did()])
  return program
def clear_state_program():
  return Return(Int(1))
if __name__==\"__main__\":
  print(\"DIDManager-W3C DID\!\")
