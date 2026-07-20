-module(stingray_ast).
-export([
    program/1,
    use_directive/3, sideway_directive/2, flow_directive/3,
    fun_decl/4, enum_decl/3, type_alias/3, struct_decl/4,
    param/3, field/3,
    int_literal/2, float_literal/2, string_literal/2, char_literal/2,
    bool_literal/2, identifier/2, type_ref/2, qualified_name/3,
    binary_op/4, unary_op/3, call_expr/3, member_access/3,
    new_expr/3, list_literal/2, set_literal/2, list_access/3,
    var_decl/4, assignment/3, while_stmt/3, return_stmt/2, expr_stmt/2,
    if_stmt/4,
    type_basic/2, type_union/3, type_param/3
]).

program(TopLevel) -> {program, TopLevel}.

use_directive(Module, Alias, {L, C}) -> {use_directive, Module, Alias, L, C}.
sideway_directive(Expr, {L, C}) -> {sideway_directive, Expr, L, C}.
flow_directive(Count, Expr, {L, C}) -> {flow_directive, Count, Expr, L, C}.

fun_decl(Name, Params, Body, {L, C}) -> {fun_decl, Name, Params, Body, L, C}.
enum_decl(Name, Values, {L, C}) -> {enum_decl, Name, Values, L, C}.
type_alias(Name, TypeExpr, {L, C}) -> {type_alias, Name, TypeExpr, L, C}.
struct_decl(Name, Fields, Extends, {L, C}) -> {struct_decl, Name, Fields, Extends, L, C}.
param(Name, Type, {L, C}) -> {param, Name, Type, L, C}.
field(Name, Type, {L, C}) -> {field, Name, Type, L, C}.

int_literal(Value, {L, C}) -> {int_literal, Value, L, C}.
float_literal(Value, {L, C}) -> {float_literal, Value, L, C}.
string_literal(Value, {L, C}) -> {string_literal, Value, L, C}.
char_literal(Value, {L, C}) -> {char_literal, Value, L, C}.
bool_literal(Value, {L, C}) -> {bool_literal, Value, L, C}.

identifier(Name, {L, C}) -> {identifier, Name, L, C}.
type_ref(Name, {L, C}) -> {type_ref, Name, L, C}.
qualified_name(Module, Name, {L, C}) -> {qualified_name, Module, Name, L, C}.

binary_op(Op, Left, Right, {L, C}) -> {binary_op, Op, Left, Right, L, C}.
unary_op(Op, Operand, {L, C}) -> {unary_op, Op, Operand, L, C}.

call_expr(Callee, Args, {L, C}) -> {call, Callee, Args, L, C}.
member_access(Object, Field, {L, C}) -> {member_access, Object, Field, L, C}.
new_expr(TypeName, Args, {L, C}) -> {new_expr, TypeName, Args, L, C}.
list_literal(Elements, {L, C}) -> {list_literal, Elements, L, C}.
set_literal(Elements, {L, C}) -> {set_literal, Elements, L, C}.
list_access(Base, Index, {L, C}) -> {list_access, Base, Index, L, C}.

var_decl(Name, Type, Init, {L, C}) -> {var_decl, Name, Type, Init, L, C}.
assignment(Target, Value, {L, C}) -> {assignment, Target, Value, L, C}.
while_stmt(Condition, Body, {L, C}) -> {while_stmt, Condition, Body, L, C}.
if_stmt(Condition, ThenBody, ElseBody, {L, C}) ->
    {if_stmt, Condition, ThenBody, ElseBody, L, C}.
return_stmt(Value, {L, C}) -> {return_stmt, Value, L, C}.
expr_stmt(Expr, {L, C}) -> {expr_stmt, Expr, L, C}.

type_basic(Name, {L, C}) -> {type_basic, Name, L, C}.
type_union(Left, Right, {L, C}) -> {type_union, Left, Right, L, C}.
type_param(Name, Param, {L, C}) -> {type_param, Name, Param, L, C}.