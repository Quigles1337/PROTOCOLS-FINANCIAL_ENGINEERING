"""SignerList-Weighted Multi-Signature"""

from pyteal import *
NUM_GLOBAL_UINTS=4
NUM_GLOBAL_BYTES=1
NUM_LOCAL_UINTS=12
NUM_LOCAL_BYTES=8

def approval_program():
  admin=Bytes("admin")
  total_lists=Bytes("total_lists")
  total_proposals=Bytes("total_proposals")
  sl_owner=Bytes("owner")
  sl_quorum=Bytes("quorum")
  sl_total_weight=Bytes("total_weight")
  sl_num_signers=Bytes("num_signers")
  sl_created_at=Bytes("created")
  sl_status=Bytes("status")
  on_creation=Seq([App.globalPut(admin,Txn.sender()),App.globalPut(total_lists,Int(0)),App.globalPut(total_proposals,Int(0)),Return(Int(1))])
  @Subroutine(TealType.uint64)
  def create_list():
    quorum=Btoi(Txn.application_args[1])
    owner=Txn.sender()
    return Seq([Assert(quorum>Int(0)),App.localPut(owner,sl_owner,owner),App.localPut(owner,sl_quorum,quorum),App.localPut(owner,sl_total_weight,Int(0)),App.localPut(owner,sl_num_signers,Int(0)),App.localPut(owner,sl_created_at,Global.round()),App.localPut(owner,sl_status,Int(0)),App.globalPut(total_lists,App.globalGet(total_lists)+Int(1)),Return(Int(1))])
  @Subroutine(TealType.uint64)
  def add_signer():
    signer_addr=Txn.application_args[1]
    weight=Btoi(Txn.application_args[2])
    caller=Txn.sender()
    owner_addr=App.localGet(caller,sl_owner)
    total_weight=App.localGet(caller,sl_total_weight)
    num_signers=App.localGet(caller,sl_num_signers)
    status=App.localGet(caller,sl_status)
    return Seq([Assert(caller==owner_addr),Assert(status==Int(0)),Assert(Len(signer_addr)==Int(32)),Assert(weight>Int(0)),App.localPut(caller,Concat(Bytes("signer_"),signer_addr),weight),App.localPut(caller,Concat(Bytes("active_"),signer_addr),Int(1)),App.localPut(caller,Concat(Bytes("added_"),signer_addr),Global.round()),App.localPut(caller,sl_total_weight,total_weight+weight),App.localPut(caller,sl_num_signers,num_signers+Int(1)),Return(Int(1))])
  @Subroutine(TealType.uint64)
  def remove_signer():
    signer_addr=Txn.application_args[1]
    caller=Txn.sender()
    owner_addr=App.localGet(caller,sl_owner)
    total_weight=App.localGet(caller,sl_total_weight)
    num_signers=App.localGet(caller,sl_num_signers)
    status=App.localGet(caller,sl_status)
    signer_weight_val=App.localGet(caller,Concat(Bytes("signer_"),signer_addr))
    signer_active_val=App.localGet(caller,Concat(Bytes("active_"),signer_addr))
    return Seq([Assert(caller==owner_addr),Assert(status==Int(0)),Assert(signer_active_val==Int(1)),App.localPut(caller,Concat(Bytes("active_"),signer_addr),Int(0)),App.localPut(caller,sl_total_weight,total_weight-signer_weight_val),App.localPut(caller,sl_num_signers,num_signers-Int(1)),Return(Int(1))])
  @Subroutine(TealType.uint64)
  def update_quorum():
    new_quorum=Btoi(Txn.application_args[1])
    caller=Txn.sender()
    owner_addr=App.localGet(caller,sl_owner)
    total_weight=App.localGet(caller,sl_total_weight)
    status=App.localGet(caller,sl_status)
    return Seq([Assert(caller==owner_addr),Assert(status==Int(0)),Assert(new_quorum>Int(0)),Assert(new_quorum<=total_weight),App.localPut(caller,sl_quorum,new_quorum),Return(Int(1))])
  @Subroutine(TealType.uint64)
  def create_proposal():
    proposal_data=Txn.application_args[1]
    expires_at=Btoi(Txn.application_args[2])
    caller=Txn.sender()
    status=App.localGet(caller,sl_status)
    proposal_id=App.globalGet(total_proposals)+Int(1)
    return Seq([Assert(status==Int(0)),Assert(expires_at>Global.round()),App.localPut(caller,Concat(Bytes("prop_id_"),Itob(proposal_id)),proposal_id),App.localPut(caller,Concat(Bytes("prop_creator_"),Itob(proposal_id)),caller),App.localPut(caller,Concat(Bytes("prop_data_"),Itob(proposal_id)),proposal_data),App.localPut(caller,Concat(Bytes("prop_approvals_"),Itob(proposal_id)),Int(0)),App.localPut(caller,Concat(Bytes("prop_status_"),Itob(proposal_id)),Int(0)),App.localPut(caller,Concat(Bytes("prop_created_"),Itob(proposal_id)),Global.round()),App.localPut(caller,Concat(Bytes("prop_expires_"),Itob(proposal_id)),expires_at),App.globalPut(total_proposals,proposal_id),Return(Int(1))])
  @Subroutine(TealType.uint64)
  def approve_proposal():
    proposal_id=Btoi(Txn.application_args[1])
    caller=Txn.sender()
    quorum=App.localGet(caller,sl_quorum)
    signer_weight_val=App.localGet(caller,Concat(Bytes("signer_"),caller))
    signer_active_val=App.localGet(caller,Concat(Bytes("active_"),caller))
    prop_approvals_val=App.localGet(caller,Concat(Bytes("prop_approvals_"),Itob(proposal_id)))
    prop_status_val=App.localGet(caller,Concat(Bytes("prop_status_"),Itob(proposal_id)))
    prop_expires_val=App.localGet(caller,Concat(Bytes("prop_expires_"),Itob(proposal_id)))
    new_approvals=prop_approvals_val+signer_weight_val
    return Seq([Assert(signer_active_val==Int(1)),Assert(prop_status_val==Int(0)),Assert(Global.round()<prop_expires_val),App.localPut(caller,Concat(Bytes("prop_approvals_"),Itob(proposal_id)),new_approvals),If(new_approvals>=quorum).Then(App.localPut(caller,Concat(Bytes("prop_status_"),Itob(proposal_id)),Int(1))),Return(Int(1))])
  @Subroutine(TealType.uint64)
  def execute_proposal():
    proposal_id=Btoi(Txn.application_args[1])
    caller=Txn.sender()
    prop_status_val=App.localGet(caller,Concat(Bytes("prop_status_"),Itob(proposal_id)))
    return Seq([Assert(prop_status_val==Int(1)),App.localPut(caller,Concat(Bytes("prop_status_"),Itob(proposal_id)),Int(2)),Return(Int(1))])
  program=Cond([Txn.application_id()==Int(0),on_creation],[Txn.on_completion()==OnComplete.OptIn,Return(Int(1))],[Txn.on_completion()==OnComplete.CloseOut,Return(Int(1))],[Txn.application_args[0]==Bytes("create"),create_list()],[Txn.application_args[0]==Bytes("add_signer"),add_signer()],[Txn.application_args[0]==Bytes("remove_signer"),remove_signer()],[Txn.application_args[0]==Bytes("update_quorum"),update_quorum()],[Txn.application_args[0]==Bytes("create_proposal"),create_proposal()],[Txn.application_args[0]==Bytes("approve"),approve_proposal()],[Txn.application_args[0]==Bytes("execute"),execute_proposal()])
  return program
def clear_state_program():
  return Return(Int(1))
if __name__=="__main__":
  print("SignerList-weighted multisig!")
