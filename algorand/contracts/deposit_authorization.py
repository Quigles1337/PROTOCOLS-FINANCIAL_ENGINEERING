"""DepositAuthorization-KYC/AML Compliance"""

from pyteal import *
NUM_GLOBAL_UINTS=4
NUM_GLOBAL_BYTES=2
NUM_LOCAL_UINTS=12
NUM_LOCAL_BYTES=6

def approval_program():
  admin=Bytes("admin")
  compliance=Bytes("compliance")
  total=Bytes("total")
  active=Bytes("active")
  auth_auth=Bytes("authorizer")
  auth_authd=Bytes("authorized")
  auth_asset=Bytes("asset")
  auth_min=Bytes("min")
  auth_max=Bytes("max")
  auth_total=Bytes("total_lim")
  auth_used=Bytes("used")
  auth_tier=Bytes("tier")
  auth_created=Bytes("created")
  auth_expires=Bytes("expires")
  auth_status=Bytes("status")
  auth_cred=Bytes("cred")
  auth_2fa=Bytes("2fa")
  on_creation=Seq([App.globalPut(admin,Txn.sender()),App.globalPut(compliance,Txn.sender()),App.globalPut(total,Int(0)),App.globalPut(active,Int(0)),Return(Int(1))])
  @Subroutine(TealType.uint64)
  def create_auth():
    addr=Txn.application_args[1]
    asset=Btoi(Txn.application_args[2])
    min_amt=Btoi(Txn.application_args[3])
    max_amt=Btoi(Txn.application_args[4])
    total_lim=Btoi(Txn.application_args[5])
    tier=Btoi(Txn.application_args[6])
    expires=Btoi(Txn.application_args[7])
    cred=Txn.application_args[8]
    twofa=Btoi(Txn.application_args[9]) if Len(Txn.application_args)>Int(9) else Int(0)
    auth=Txn.sender()
    comp=App.globalGet(compliance)
    return Seq([Assert(Or(auth==comp,auth==App.globalGet(admin))),Assert(Len(addr)==Int(32)),Assert(max_amt>Int(0)),Assert(total_lim>=max_amt),Assert(min_amt<=max_amt),Assert(tier<=Int(3)),Assert(expires>Global.round()),App.localPut(auth,auth_auth,auth),App.localPut(auth,auth_authd,addr),App.localPut(auth,auth_asset,Itob(asset)),App.localPut(auth,auth_min,min_amt),App.localPut(auth,auth_max,max_amt),App.localPut(auth,auth_total,total_lim),App.localPut(auth,auth_used,Int(0)),App.localPut(auth,auth_tier,tier),App.localPut(auth,auth_created,Global.round()),App.localPut(auth,auth_expires,expires),App.localPut(auth,auth_status,Int(0)),App.localPut(auth,auth_cred,cred),App.localPut(auth,auth_2fa,twofa),App.globalPut(total,App.globalGet(total)+Int(1)),App.globalPut(active,App.globalGet(active)+Int(1)),Return(Int(1))])
  @Subroutine(TealType.uint64)
  def auth_deposit():
    amt=Btoi(Txn.application_args[2])
    code=Txn.application_args[3] if Len(Txn.application_args)>Int(3) else Bytes("")
    caller=Txn.sender()
    min_a=App.localGet(caller,auth_min)
    max_a=App.localGet(caller,auth_max)
    total_l=App.localGet(caller,auth_total)
    used=App.localGet(caller,auth_used)
    status=App.localGet(caller,auth_status)
    exp=App.localGet(caller,auth_expires)
    twofa_req=App.localGet(caller,auth_2fa)
    new_used=used+amt
    return Seq([Assert(status==Int(0)),Assert(Global.round()<exp),Assert(amt>=min_a),Assert(amt<=max_a),Assert(new_used<=total_l),If(twofa_req==Int(1)).Then(Assert(Len(code)>Int(0))),App.localPut(caller,auth_used,new_used),Return(Int(1))])
  @Subroutine(TealType.uint64)
  def update_lim():
    new_max=Btoi(Txn.application_args[2])
    new_tot=Btoi(Txn.application_args[3])
    caller=Txn.sender()
    authorizer=App.localGet(caller,auth_auth)
    comp=App.globalGet(compliance)
    status=App.localGet(caller,auth_status)
    used=App.localGet(caller,auth_used)
    return Seq([Assert(Or(caller==authorizer,caller==comp,caller==App.globalGet(admin))),Assert(status==Int(0)),Assert(new_max>Int(0)),Assert(new_tot>=new_max),Assert(new_tot>=used),App.localPut(caller,auth_max,new_max),App.localPut(caller,auth_total,new_tot),Return(Int(1))])
  @Subroutine(TealType.uint64)
  def extend_exp():
    new_exp=Btoi(Txn.application_args[2])
    caller=Txn.sender()
    authorizer=App.localGet(caller,auth_auth)
    comp=App.globalGet(compliance)
    status=App.localGet(caller,auth_status)
    curr_exp=App.localGet(caller,auth_expires)
    return Seq([Assert(Or(caller==authorizer,caller==comp,caller==App.globalGet(admin))),Assert(status==Int(0)),Assert(new_exp>curr_exp),App.localPut(caller,auth_expires,new_exp),Return(Int(1))])
  @Subroutine(TealType.uint64)
  def suspend():
    caller=Txn.sender()
    authorizer=App.localGet(caller,auth_auth)
    comp=App.globalGet(compliance)
    status=App.localGet(caller,auth_status)
    return Seq([Assert(Or(caller==authorizer,caller==comp,caller==App.globalGet(admin))),Assert(status==Int(0)),App.localPut(caller,auth_status,Int(1)),App.globalPut(active,App.globalGet(active)-Int(1)),Return(Int(1))])
  @Subroutine(TealType.uint64)
  def resume():
    caller=Txn.sender()
    authorizer=App.localGet(caller,auth_auth)
    comp=App.globalGet(compliance)
    status=App.localGet(caller,auth_status)
    return Seq([Assert(Or(caller==authorizer,caller==comp,caller==App.globalGet(admin))),Assert(status==Int(1)),App.localPut(caller,auth_status,Int(0)),App.globalPut(active,App.globalGet(active)+Int(1)),Return(Int(1))])
  @Subroutine(TealType.uint64)
  def revoke():
    caller=Txn.sender()
    authorizer=App.localGet(caller,auth_auth)
    comp=App.globalGet(compliance)
    status=App.localGet(caller,auth_status)
    return Seq([Assert(Or(caller==authorizer,caller==comp,caller==App.globalGet(admin))),Assert(status!=Int(2)),App.localPut(caller,auth_status,Int(2)),If(status==Int(0)).Then(App.globalPut(active,App.globalGet(active)-Int(1))),Return(Int(1))])
  @Subroutine(TealType.uint64)
  def update_comp():
    new_off=Txn.application_args[1]
    caller=Txn.sender()
    return Seq([Assert(caller==App.globalGet(admin)),Assert(Len(new_off)==Int(32)),App.globalPut(compliance,new_off),Return(Int(1))])
  @Subroutine(TealType.uint64)
  def get_info():
    return Return(Int(1))
  program=Cond([Txn.application_id()==Int(0),on_creation],[Txn.on_completion()==OnComplete.OptIn,Return(Int(1))],[Txn.on_completion()==OnComplete.CloseOut,Return(Int(1))],[Txn.application_args[0]==Bytes("create"),create_auth()],[Txn.application_args[0]==Bytes("authorize"),auth_deposit()],[Txn.application_args[0]==Bytes("update_limits"),update_lim()],[Txn.application_args[0]==Bytes("extend"),extend_exp()],[Txn.application_args[0]==Bytes("suspend"),suspend()],[Txn.application_args[0]==Bytes("resume"),resume()],[Txn.application_args[0]==Bytes("revoke"),revoke()],[Txn.application_args[0]==Bytes("update_officer"),update_comp()],[Txn.application_args[0]==Bytes("get_info"),get_info()])
  return program
def clear_state_program():
  return Return(Int(1))
if __name__=="__main__":
  print("DepositAuthorization-KYC/AML compliance!")
