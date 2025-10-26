from pyteal import *

NUM_GLOBAL_UINTS = 5
NUM_GLOBAL_BYTES = 1
NUM_LOCAL_UINTS = 15
NUM_LOCAL_BYTES = 5

def approval_program():
    admin = Bytes("admin")
    next_order_id = Bytes("next_ord_id")
    total_orders = Bytes("total_ords")
    total_volume = Bytes("total_vol")
    total_trades = Bytes("total_trades")
    ord_id = Bytes("id")
    ord_creator = Bytes("creator")
    ord_base_token = Bytes("base_token")
    ord_quote_token = Bytes("quote_token")
    ord_side = Bytes("side")
    ord_order_type = Bytes("type")
    ord_amount = Bytes("amount")
    ord_filled = Bytes("filled")
    ord_price = Bytes("price")
    ord_status = Bytes("status")
    ord_created_at = Bytes("created")
    ord_updated_at = Bytes("updated")
    ord_expires_at = Bytes("expires")
    ord_min_fill = Bytes("min_fill")
    ord_post_only = Bytes("post_only")
    
    on_creation = Seq([App.globalPut(admin, Txn.sender()), App.globalPut(next_order_id, Int(1)), App.globalPut(total_orders, Int(0)), App.globalPut(total_volume, Int(0)), App.globalPut(total_trades, Int(0)), Return(Int(1))])
    
    @Subroutine(TealType.uint64)
    def create_order():
        base_token = Btoi(Txn.application_args[1])
        quote_token = Btoi(Txn.application_args[2])
        side = Btoi(Txn.application_args[3])
        order_type = Btoi(Txn.application_args[4])
        amount = Btoi(Txn.application_args[5])
        price = Btoi(Txn.application_args[6])
        expires_at = Btoi(Txn.application_args[7])
        min_fill = Btoi(Txn.application_args[8]) if Len(Txn.application_args) > Int(8) else Int(0)
        post_only = Btoi(Txn.application_args[9]) if Len(Txn.application_args) > Int(9) else Int(0)
        order_id = App.globalGet(next_order_id)
        creator = Txn.sender()
        return Seq([Assert(amount > Int(0)), Assert(Or(side == Int(0), side == Int(1))), Assert(Or(order_type == Int(0), order_type == Int(1))), Assert(base_token != quote_token), If(order_type == Int(0)).Then(Assert(price > Int(0))), If(expires_at > Int(0)).Then(Assert(expires_at > Global.round())), If(post_only == Int(1)).Then(Assert(order_type == Int(0))), App.localPut(creator, ord_id, order_id), App.localPut(creator, ord_creator, creator), App.localPut(creator, ord_base_token, Itob(base_token)), App.localPut(creator, ord_quote_token, Itob(quote_token)), App.localPut(creator, ord_side, side), App.localPut(creator, ord_order_type, order_type), App.localPut(creator, ord_amount, amount), App.localPut(creator, ord_filled, Int(0)), App.localPut(creator, ord_price, price), App.localPut(creator, ord_status, Int(0)), App.localPut(creator, ord_created_at, Global.round()), App.localPut(creator, ord_updated_at, Global.round()), App.localPut(creator, ord_expires_at, expires_at), App.localPut(creator, ord_min_fill, min_fill), App.localPut(creator, ord_post_only, post_only), App.globalPut(next_order_id, order_id + Int(1)), App.globalPut(total_orders, App.globalGet(total_orders) + Int(1)), Return(Int(1))])
    
    @Subroutine(TealType.uint64)
    def fill_order():
        order_id_arg = Btoi(Txn.application_args[1])
        fill_amount = Btoi(Txn.application_args[2])
        max_price = Btoi(Txn.application_args[3])
        caller = Txn.sender()
        side_val = App.localGet(caller, ord_side)
        amount = App.localGet(caller, ord_amount)
        filled = App.localGet(caller, ord_filled)
        price_val = App.localGet(caller, ord_price)
        status = App.localGet(caller, ord_status)
        expires_at_val = App.localGet(caller, ord_expires_at)
        min_fill_val = App.localGet(caller, ord_min_fill)
        remaining = amount - filled
        new_filled = filled + fill_amount
        return Seq([Assert(status == Int(0)), If(expires_at_val > Int(0)).Then(Assert(Global.round() < expires_at_val)), Assert(fill_amount > Int(0)), Assert(fill_amount <= remaining), If(side_val == Int(0)).Then(Assert(max_price >= price_val)), If(side_val == Int(1)).Then(Assert(max_price <= price_val)), If(min_fill_val > Int(0)).Then(Assert(fill_amount >= min_fill_val)), App.localPut(caller, ord_filled, new_filled), App.localPut(caller, ord_updated_at, Global.round()), If(new_filled >= amount).Then(App.localPut(caller, ord_status, Int(1))), App.globalPut(total_volume, App.globalGet(total_volume) + fill_amount), App.globalPut(total_trades, App.globalGet(total_trades) + Int(1)), Return(Int(1))])
    
    @Subroutine(TealType.uint64)
    def cancel_order():
        order_id_arg = Btoi(Txn.application_args[1])
        caller = Txn.sender()
        creator = App.localGet(caller, ord_creator)
        status = App.localGet(caller, ord_status)
        return Seq([Assert(caller == creator), Assert(status == Int(0)), App.localPut(caller, ord_status, Int(2)), App.localPut(caller, ord_updated_at, Global.round()), Return(Int(1))])
    
    @Subroutine(TealType.uint64)
    def mark_expired():
        order_id_arg = Btoi(Txn.application_args[1])
        caller = Txn.sender()
        status = App.localGet(caller, ord_status)
        expires_at_val = App.localGet(caller, ord_expires_at)
        return Seq([Assert(status == Int(0)), Assert(expires_at_val > Int(0)), Assert(Global.round() >= expires_at_val), App.localPut(caller, ord_status, Int(3)), App.localPut(caller, ord_updated_at, Global.round()), Return(Int(1))])
    
    @Subroutine(TealType.uint64)
    def update_price():
        order_id_arg = Btoi(Txn.application_args[1])
        new_price = Btoi(Txn.application_args[2])
        caller = Txn.sender()
        creator = App.localGet(caller, ord_creator)
        order_type_val = App.localGet(caller, ord_order_type)
        status = App.localGet(caller, ord_status)
        return Seq([Assert(caller == creator), Assert(order_type_val == Int(0)), Assert(status == Int(0)), Assert(new_price > Int(0)), App.localPut(caller, ord_price, new_price), App.localPut(caller, ord_updated_at, Global.round()), Return(Int(1))])
    
    program = Cond([Txn.application_id() == Int(0), on_creation], [Txn.on_completion() == OnComplete.OptIn, Return(Int(1))], [Txn.on_completion() == OnComplete.CloseOut, Return(Int(1))], [Txn.application_args[0] == Bytes("create"), create_order()], [Txn.application_args[0] == Bytes("fill"), fill_order()], [Txn.application_args[0] == Bytes("cancel"), cancel_order()], [Txn.application_args[0] == Bytes("mark_expired"), mark_expired()], [Txn.application_args[0] == Bytes("update_price"), update_price()])
    return program

def clear_state_program():
    return Return(Int(1))

if __name__ == "__main__":
    print("DEXOrders contract")
