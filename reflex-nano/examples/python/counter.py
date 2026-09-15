"""Python constructs native definitions; Rust owns and executes them."""
from reflex_nano import App, Schema, Program, Action, Expr, Node

schema=Schema()
schema.field("count","int","0")
schema.computed("doubled",Expr.state("count").binary("mul",Expr.literal("2")))
schema.event("increment",Program([
    Action.set("count",Expr.state("count").binary("add",Expr.literal("1")))
]))
app=App(schema)
app.add_page("/","Native counter",Node.element("main",[
    Node.text(Expr.state("count")),
    Node.element("button",[Node.text(Expr.literal('"Increment"'))])
        .on("click","increment",[],False),
]))

if __name__=="__main__":
    app.run()
