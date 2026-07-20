-module(stingray_parser).
-export([parse/1, parse_file/1, compile/2, compile_file/2]).

%% ============================================================================
%% Public API
%% ============================================================================

-spec parse_file(file:filename()) -> {ok, tuple()} | {error, term()}.
parse_file(Path) ->
    case file:read_file(Path) of
        {ok, Bin} ->
            case stingray_lexer:tokenize(Bin) of
                {error, _} = Err -> Err;
                Tokens -> parse(Tokens)
            end;
        {error, _} = Err -> Err
    end.

-spec parse([tuple()]) -> {ok, tuple()} | {error, term()}.
parse(Tokens) ->
    try
        {ok, Program, []} = parse_program(Tokens),
        {ok, Program}
    catch
        throw:{error, _} = Err -> Err;
        error:badmatch -> {error, {0, 0, parse_failed}};
        error:Reason -> {error, {0, 0, {parse_error, Reason}}}
    end.

-spec compile(file:filename(), file:filename()) -> {ok, atom()} | {error, term()}.
compile(InputPath, OutputPath) ->
    case parse_file(InputPath) of
        {error, _} = Err -> Err;
        {ok, AST} ->
            case stingray_codegen:compile(AST, [{module, stingray_output}]) of
                {ok, Module, BeamBinary} ->
                    file:write_file(OutputPath, BeamBinary),
                    {ok, Module};
                {error, _} = Err -> Err
            end
    end.

-spec compile_file(file:filename(), file:filename()) -> {ok, atom()} | {error, term()}.
compile_file(InputPath, OutputPath) ->
    compile(InputPath, OutputPath).

%% ============================================================================
%% Program: top-level declarations
%% ============================================================================

parse_program(Tokens) ->
    parse_program(Tokens, []).

parse_program([], Acc) ->
    {ok, stingray_ast:program(lists:reverse(Acc)), []};
parse_program(Tokens, Acc) ->
    {TopLevel, Rest} = parse_top_level(Tokens),
    parse_program(Rest, [TopLevel | Acc]).

parse_top_level([{directive_use, <<"use">>, L, C} | Rest]) ->
    {Module, Rest1} = expect_name(Rest),
    {_, Rest2} = expect(keyword_as, Rest1),
    {AliasName, Rest3} = expect_name(Rest2),
    {stingray_ast:use_directive(Module, AliasName, {L, C}), Rest3};

parse_top_level([{directive_sideway, <<"sideway">>, L, C} | Rest]) ->
    {Expr, Rest1} = parse_expression(Rest),
    {stingray_ast:sideway_directive(Expr, {L, C}), Rest1};

parse_top_level([{directive_flow, Count, L, C} | Rest]) ->
    {Expr, Rest1} = parse_expression(Rest),
    {stingray_ast:flow_directive(Count, Expr, {L, C}), Rest1};

parse_top_level([{keyword_fun, <<"fun">>, L, C} | Rest]) ->
    parse_fun(L, C, Rest);

parse_top_level([{keyword_enum, <<"enum">>, L, C} | Rest]) ->
    parse_enum(L, C, Rest);

parse_top_level([{keyword_type, <<"type">>, L, C} | Rest]) ->
    parse_type_decl(L, C, Rest);

%% Top-level assignment (e.g., AtomEnum = math.anyEnum)
parse_top_level(Tokens) ->
    {Expr, Rest} = parse_expression(Tokens),
    case Expr of
        {identifier, Name, EL, EC} ->
            parse_top_level_assignment(Name, EL, EC, Rest);
        {type_ref, Name, EL, EC} ->
            parse_top_level_assignment(Name, EL, EC, Rest);
        _ ->
            throw({error, {1, 1, invalid_top_level}})
    end.

parse_top_level_assignment(Name, EL, EC, Rest) ->
    case Rest of
        [{equal, _, _, _} | Rest1] ->
            {ValueExpr, Rest2} = parse_expression(Rest1),
            {stingray_ast:assignment(stingray_ast:identifier(Name, {EL, EC}), ValueExpr, {EL, EC}), Rest2};
        _ ->
            throw({error, {EL, EC, {expected_token, equal}}})
    end.

%% ============================================================================
%% fun declaration
%% ============================================================================

parse_fun(L, C, [{lparen, <<"(">>, _, _} | Rest]) ->
    %% Forward declaration: fun (name)
    {Name, Rest1} = expect(identifier, Rest),
    {_, Rest2} = expect(rparen, Rest1),
    {stingray_ast:fun_decl(Name, [], [], {L, C}), Rest2};

parse_fun(L, C, Rest) ->
    %% fun name(params) { body }
    {Name, Rest1} = expect(identifier, Rest),
    {_, Rest2} = expect(lparen, Rest1),
    {Params, Rest3} = parse_params(Rest2),
    {_, Rest4} = expect(rparen, Rest3),
    case Rest4 of
        [{lbrace, _, _, _} | _] ->
            {Body, Rest5} = parse_block(Rest4),
            {stingray_ast:fun_decl(Name, Params, Body, {L, C}), Rest5};
        _ ->
            %% Forward declaration without body
            {stingray_ast:fun_decl(Name, Params, [], {L, C}), Rest4}
    end.

parse_params([{rparen, _, _, _} | _] = Tokens) ->
    {[], Tokens};
parse_params(Tokens) ->
    parse_params(Tokens, []).

parse_params([{rparen, _, _, _} | _] = Tokens, Acc) ->
    {lists:reverse(Acc), Tokens};
parse_params(Tokens, Acc) ->
    {Name, Rest1} = expect(identifier, Tokens),
    {_, Rest2} = expect(colon, Rest1),
    {Type, Rest3} = parse_type(Rest2),
    Param = stingray_ast:param(Name, Type, token_loc(Name)),
    case Rest3 of
        [{comma, _, _, _} | Rest4] -> parse_params(Rest4, [Param | Acc]);
        _ -> {lists:reverse([Param | Acc]), Rest3}
    end.

%% ============================================================================
%% enum declaration
%% ============================================================================

parse_enum(L, C, Rest) ->
    {Name, Rest1} = expect(identifier, Rest),
    {_, Rest2} = expect(lbrace, Rest1),
    {Values, Rest3} = parse_enum_values(Rest2, []),
    {stingray_ast:enum_decl(Name, Values, {L, C}), Rest3}.

parse_enum_values([{rbrace, _, _, _} | Rest], Acc) ->
    {lists:reverse(Acc), Rest};
parse_enum_values([{type_identifier, Name, _, _} | Rest], Acc) ->
    parse_enum_values(Rest, [Name | Acc]);
parse_enum_values([{identifier, Name, _, _} | Rest], Acc) ->
    parse_enum_values(Rest, [Name | Acc]);
parse_enum_values([], _Acc) ->
    throw({error, {0, 0, unterminated_enum}}).

%% ============================================================================
%% type declaration
%% ============================================================================

parse_type_decl(L, C, [{keyword_struct, <<"struct">>, _, _} | Rest]) ->
    parse_struct(L, C, Rest);
parse_type_decl(L, C, Rest) ->
    %% type alias: type Name (- TypeExpr
    {Name, Rest1} = expect(type_identifier, Rest),
    {_, Rest2} = expect(lparen, Rest1),
    {_, Rest3} = expect(minus, Rest2),
    {TypeExpr, Rest4} = parse_type_union(Rest3),
    {stingray_ast:type_alias(Name, TypeExpr, {L, C}), Rest4}.

parse_struct(L, C, Rest) ->
    {Name, Rest1} = expect(type_identifier, Rest),
    case Rest1 of
        [{lbrace, _, _, _} | _] ->
            %% type struct Name { fields ... methods ... }
            {_, Rest2} = expect(lbrace, Rest1),
            {Fields, Methods, Rest3} = parse_struct_body(Rest2, [], []),
            {_, Rest4} = expect(rbrace, Rest3),
            {stingray_ast:struct_decl(Name, Fields, undefined, {L, C}), Rest4};
        [{lparen, _, _, _} | _] ->
            %% type struct Name (- TypeExpr (extends or inline type)
            {_, Rest2} = expect(lparen, Rest1),
            {_, Rest3} = expect(minus, Rest2),
            {TypeExpr, Rest4} = parse_type_union(Rest3),
            case TypeExpr of
                {type_struct_fields, Fields, _, _} ->
                    {stingray_ast:struct_decl(Name, Fields, undefined, {L, C}), Rest4};
                _ ->
                    {stingray_ast:struct_decl(Name, [], TypeExpr, {L, C}), Rest4}
            end;
        _ ->
            throw({error, {L, C, expected_struct_body}})
    end.

parse_struct_body([{rbrace, _, _, _} | _] = Tokens, Fields, Methods) ->
    {lists:reverse(Fields), lists:reverse(Methods), Tokens};
parse_struct_body([], _Fields, _Methods) ->
    throw({error, {0, 0, unterminated_struct}});
parse_struct_body([{keyword_fun, _, _, _} | _] = Tokens, Fields, Methods) ->
    %% Parse the function inside the struct
    {FunDecl, Rest1} = parse_top_level(Tokens),
    parse_struct_body(Rest1, Fields, [FunDecl | Methods]);
parse_struct_body(Tokens, Fields, Methods) ->
    {Type, Rest1} = parse_type(Tokens),
    {Name, Rest2} = expect(identifier, Rest1),
    F = stingray_ast:field(Name, Type, token_loc(Name)),
    case Rest2 of
        [{comma, _, _, _} | Rest3] -> parse_struct_body(Rest3, [F | Fields], Methods);
        [{dot, _, _, _} | Rest3] -> parse_struct_body(Rest3, [F | Fields], Methods);
        _ -> parse_struct_body(Rest2, [F | Fields], Methods)
    end.

%% ============================================================================
%% Type expressions
%% ============================================================================

parse_type(Tokens) ->
    parse_type_basic(Tokens).

parse_type_basic([{type_identifier, Name, L, C} | Rest]) ->
    parse_type_basic_rest(Name, L, C, Rest);
parse_type_basic([{identifier, Name, L, C} | Rest]) ->
    parse_type_basic_rest(Name, L, C, Rest);
parse_type_basic([{lparen, _, _, _} | Rest]) ->
    {Type, Rest1} = parse_type_union(Rest),
    {_, Rest2} = expect(rparen, Rest1),
    {Type, Rest2}.

parse_type_basic_rest(Name, L, C, [{dot, _, _, _}, {type_identifier, ParamName, _, _} | Rest2]) ->
    {stingray_ast:type_param(Name, stingray_ast:type_basic(ParamName, {0, 0}), {L, C}), Rest2};
parse_type_basic_rest(Name, L, C, [{colon, _, _, _}, {identifier, FieldName, _, _} | Rest2]) ->
    %% Type:name syntax in struct field definitions
    {stingray_ast:type_param(Name, stingray_ast:identifier(FieldName, {0, 0}), {L, C}), Rest2};
parse_type_basic_rest(Name, L, C, Rest) ->
    {stingray_ast:type_basic(Name, {L, C}), Rest}.

parse_type_union(Tokens) ->
    {Left, Rest} = parse_type_basic(Tokens),
    case Rest of
        [{pipe_pipe, _, _, _} | Rest1] ->
            {Right, Rest2} = parse_type_union(Rest1),
            {stingray_ast:type_union(Left, Right, token_loc(Left)), Rest2};
        [{pipe, _, _, _} | Rest1] ->
            {Right, Rest2} = parse_type_union(Rest1),
            {stingray_ast:type_union(Left, Right, token_loc(Left)), Rest2};
        [{ampersand, _, _, _} | Rest1] ->
            {Right, Rest2} = parse_type_union(Rest1),
            {stingray_ast:type_union(Left, Right, token_loc(Left)), Rest2};
        _ ->
            {Left, Rest}
    end.

%% ============================================================================
%% Block: { stmt1 stmt2 ... }
%% ============================================================================

parse_block([{lbrace, _, _, _} | Rest]) ->
    parse_block_stmts(Rest, []).

parse_block_stmts([{rbrace, _, _, _} | Rest], Acc) ->
    {lists:reverse(Acc), Rest};
parse_block_stmts([], _Acc) ->
    throw({error, {0, 0, unterminated_block}});
parse_block_stmts(Tokens, Acc) ->
    {Stmt, Rest} = parse_statement(Tokens),
    parse_block_stmts(Rest, [Stmt | Acc]).

%% ============================================================================
%% Statements
%% ============================================================================

parse_statement([{keyword_while, <<"while">>, L, C} | Rest]) ->
    {Cond, Rest1} = parse_expression(Rest),
    {Body, Rest2} = parse_block(Rest1),
    {stingray_ast:while_stmt(Cond, Body, {L, C}), Rest2};

parse_statement([{keyword_if, <<"if">>, L, C} | Rest]) ->
    {Cond, Rest1} = parse_expression(Rest),
    {ThenBody, Rest2} = parse_block(Rest1),
    case Rest2 of
        [{keyword_else, <<"else">>, _, _} | Rest3] ->
            {ElseBody, Rest4} = parse_block(Rest3),
            {stingray_ast:if_stmt(Cond, ThenBody, ElseBody, {L, C}), Rest4};
        _ ->
            {stingray_ast:if_stmt(Cond, ThenBody, [], {L, C}), Rest2}
    end;

parse_statement([{keyword_return, <<"return">>, L, C} | Rest]) ->
    case Rest of
        [{rbrace, _, _, _} | _] ->
            {stingray_ast:return_stmt({int_literal, 0, L, C}, {L, C}), Rest};
        _ ->
            {Expr, Rest1} = parse_expression(Rest),
            {stingray_ast:return_stmt(Expr, {L, C}), Rest1}
    end;

parse_statement([{directive_sideway, <<"sideway">>, L, C} | Rest]) ->
    {Expr, Rest1} = parse_expression(Rest),
    {stingray_ast:sideway_directive(Expr, {L, C}), Rest1};

parse_statement([{directive_flow, Count, L, C} | Rest]) ->
    {Expr, Rest1} = parse_expression(Rest),
    {stingray_ast:flow_directive(Count, Expr, {L, C}), Rest1};

%% var_decl: name: Type = expr or name = expr
parse_statement([{identifier, Name, L, C}, {colon, _, _, _} | _] = Tokens) ->
    parse_var_decltyped(Tokens);

%% Simple assignment: name = expr or name.field = expr
parse_statement([{identifier, _, _, _} | _] = Tokens) ->
    {Expr, Rest} = parse_expression(Tokens),
    case Rest of
        [{equal, _, _, _} | Rest1] ->
            {Value, Rest2} = parse_expression(Rest1),
            {stingray_ast:assignment(Expr, Value, token_loc(Expr)), Rest2};
        _ ->
            {stingray_ast:expr_stmt(Expr, token_loc(Expr)), Rest}
    end;

parse_statement(Tokens) ->
    {Expr, Rest} = parse_expression(Tokens),
    {stingray_ast:expr_stmt(Expr, token_loc(Expr)), Rest}.

parse_var_decltyped([{identifier, Name, L, C}, {colon, _, _, _} | Rest]) ->
    {Type, Rest1} = parse_type(Rest),
    case Rest1 of
        [{equal, _, _, _} | Rest2] ->
            {Init, Rest3} = parse_expression(Rest2),
            {stingray_ast:var_decl(Name, Type, Init, {L, C}), Rest3};
        _ ->
            {stingray_ast:var_decl(Name, Type, stingray_ast:bool_literal(false, {0, 0}), {L, C}), Rest1}
    end.

%% ============================================================================
%% Expressions — precedence climbing
%% ============================================================================

%% || and or (lowest precedence)
parse_expression(Tokens) ->
    parse_expr_pipe(Tokens).

parse_expr_pipe(Tokens) ->
    {Left, Rest} = parse_expr_and(Tokens),
    case Rest of
        [{pipe_pipe, _, _, _} | Rest1] ->
            {Right, Rest2} = parse_expr_pipe(Rest1),
            {stingray_ast:binary_op('or', Left, Right, token_loc(Left)), Rest2};
        _ ->
            {Left, Rest}
    end.

parse_expr_and(Tokens) ->
    {Left, Rest} = parse_expr_ampersand(Tokens),
    case Rest of
        [{and_and, _, _, _} | Rest1] ->
            {Right, Rest2} = parse_expr_and(Rest1),
            {stingray_ast:binary_op('and', Left, Right, token_loc(Left)), Rest2};
        _ ->
            {Left, Rest}
    end.

parse_expr_ampersand(Tokens) ->
    {Left, Rest} = parse_expr_comparison(Tokens),
    case Rest of
        [{pipe, _, _, _} | Rest1] ->
            {Right, Rest2} = parse_expr_ampersand(Rest1),
            {stingray_ast:binary_op('|', Left, Right, token_loc(Left)), Rest2};
        [{ampersand, _, _, _} | Rest1] ->
            {Right, Rest2} = parse_expr_ampersand(Rest1),
            {stingray_ast:binary_op('&', Left, Right, token_loc(Left)), Rest2};
        _ ->
            {Left, Rest}
    end.

parse_expr_comparison(Tokens) ->
    {Left, Rest} = parse_expr_additive(Tokens),
    case Rest of
        [{equal_equal, _, _, _} | Rest1] ->
            {Right, Rest2} = parse_expr_additive(Rest1),
            {stingray_ast:binary_op('==', Left, Right, token_loc(Left)), Rest2};
        [{bang_equal, _, _, _} | Rest1] ->
            {Right, Rest2} = parse_expr_additive(Rest1),
            {stingray_ast:binary_op('/=', Left, Right, token_loc(Left)), Rest2};
        [{less, _, _, _} | Rest1] ->
            {Right, Rest2} = parse_expr_additive(Rest1),
            {stingray_ast:binary_op('<', Left, Right, token_loc(Left)), Rest2};
        [{less_equal, _, _, _} | Rest1] ->
            {Right, Rest2} = parse_expr_additive(Rest1),
            {stingray_ast:binary_op('=<', Left, Right, token_loc(Left)), Rest2};
        [{greater, _, _, _} | Rest1] ->
            {Right, Rest2} = parse_expr_additive(Rest1),
            {stingray_ast:binary_op('>', Left, Right, token_loc(Left)), Rest2};
        [{greater_equal, _, _, _} | Rest1] ->
            {Right, Rest2} = parse_expr_additive(Rest1),
            {stingray_ast:binary_op('>=', Left, Right, token_loc(Left)), Rest2};
        _ ->
            {Left, Rest}
    end.

parse_expr_additive(Tokens) ->
    {Left, Rest} = parse_expr_mul(Tokens),
    parse_expr_additive_loop(Rest, Left).

parse_expr_additive_loop([{plus, _, _, _} | Rest], Left) ->
    {Right, Rest1} = parse_expr_mul(Rest),
    parse_expr_additive_loop(Rest1, stingray_ast:binary_op('+', Left, Right, token_loc(Left)));
parse_expr_additive_loop([{minus, _, _, _} | Rest], Left) ->
    {Right, Rest1} = parse_expr_mul(Rest),
    parse_expr_additive_loop(Rest1, stingray_ast:binary_op('-', Left, Right, token_loc(Left)));
parse_expr_additive_loop(Rest, Left) ->
    {Left, Rest}.

parse_expr_mul(Tokens) ->
    {Left, Rest} = parse_expr_unary(Tokens),
    parse_expr_mul_loop(Rest, Left).

parse_expr_mul_loop([{star, _, _, _} | Rest], Left) ->
    {Right, Rest1} = parse_expr_unary(Rest),
    parse_expr_mul_loop(Rest1, stingray_ast:binary_op('*', Left, Right, token_loc(Left)));
parse_expr_mul_loop([{slash, _, _, _} | Rest], Left) ->
    {Right, Rest1} = parse_expr_unary(Rest),
    parse_expr_mul_loop(Rest1, stingray_ast:binary_op('/', Left, Right, token_loc(Left)));
parse_expr_mul_loop(Rest, Left) ->
    {Left, Rest}.

parse_expr_unary([{minus, _, L, C} | Rest]) ->
    {Operand, Rest1} = parse_expr_postfix(Rest),
    {stingray_ast:unary_op('-', Operand, {L, C}), Rest1};
parse_expr_unary([{bang, _, L, C} | Rest]) ->
    {Operand, Rest1} = parse_expr_postfix(Rest),
    {stingray_ast:unary_op('not', Operand, {L, C}), Rest1};
parse_expr_unary([{keyword_not, _, L, C} | Rest]) ->
    {Operand, Rest1} = parse_expr_postfix(Rest),
    {stingray_ast:unary_op('not', Operand, {L, C}), Rest1};
parse_expr_unary([{plus_plus, _, _, _} | Rest]) ->
    %% prefix ++
    {Operand, Rest1} = parse_expr_postfix(Rest),
    {stingray_ast:unary_op('++pre', Operand, token_loc(Operand)), Rest1};
parse_expr_unary(Tokens) ->
    parse_expr_postfix(Tokens).

%% postfix: .field, (args), ++
parse_expr_postfix(Tokens) ->
    {Base, Rest} = parse_expr_primary(Tokens),
    parse_expr_postfix_loop(Rest, Base).

parse_expr_postfix_loop([{dot, _, _, _}, {identifier, Field, _, _} | Rest], Base) ->
    parse_expr_postfix_loop(Rest, stingray_ast:member_access(Base, Field, token_loc(Base)));
parse_expr_postfix_loop([{dot, _, _, _}, {type_identifier, Field, _, _} | Rest], Base) ->
    parse_expr_postfix_loop(Rest, stingray_ast:member_access(Base, Field, token_loc(Base)));
parse_expr_postfix_loop([{lparen, _, _, _} | Rest], Base) ->
    {Args, Rest1} = parse_args(Rest),
    {_, Rest2} = expect(rparen, Rest1),
    parse_expr_postfix_loop(Rest2, stingray_ast:call_expr(Base, Args, token_loc(Base)));
parse_expr_postfix_loop([{plus_plus, _, _, _} | Rest], Base) ->
    parse_expr_postfix_loop(Rest, stingray_ast:unary_op('++post', Base, token_loc(Base)));
parse_expr_postfix_loop([{lbracket, _, _, _} | Rest], Base) ->
    {Index, Rest1} = parse_expression(Rest),
    {_, Rest2} = expect(rbracket, Rest1),
    parse_expr_postfix_loop(Rest2, stingray_ast:list_access(Base, Index, token_loc(Base)));
parse_expr_postfix_loop(Rest, Base) ->
    {Base, Rest}.

%% ============================================================================
%% Primary expressions
%% ============================================================================

parse_expr_primary([{integer, Value, L, C} | Rest]) ->
    {stingray_ast:int_literal(Value, {L, C}), Rest};
parse_expr_primary([{float, Value, L, C} | Rest]) ->
    {stingray_ast:float_literal(Value, {L, C}), Rest};
parse_expr_primary([{string, Value, L, C} | Rest]) ->
    {stingray_ast:string_literal(Value, {L, C}), Rest};
parse_expr_primary([{char, Value, L, C} | Rest]) ->
    {stingray_ast:char_literal(Value, {L, C}), Rest};
parse_expr_primary([{keyword_true, _, L, C} | Rest]) ->
    {stingray_ast:bool_literal(true, {L, C}), Rest};
parse_expr_primary([{keyword_false, _, L, C} | Rest]) ->
    {stingray_ast:bool_literal(false, {L, C}), Rest};

%% Identifier or qualified name
parse_expr_primary([{identifier, Name, L, C}, {dot, _, _, _}, {identifier, Field, _, _} | Rest]) ->
    %% Could be module.name or obj.field — treat as member access for now
    Base = stingray_ast:identifier(Name, {L, C}),
    {stingray_ast:member_access(Base, Field, {L, C}), Rest};
parse_expr_primary([{identifier, Name, L, C} | Rest]) ->
    {stingray_ast:identifier(Name, {L, C}), Rest};

%% Type reference used as expression (e.g., User.new(...))
parse_expr_primary([{type_identifier, Name, L, C}, {dot, _, _, _}, {keyword_new, _, _, _} | Rest]) ->
    {_, Rest1} = expect(lparen, Rest),
    {Args, Rest2} = parse_args(Rest1),
    {_, Rest3} = expect(rparen, Rest2),
    {stingray_ast:new_expr(Name, Args, {L, C}), Rest3};
parse_expr_primary([{type_identifier, Name, L, C} | Rest]) ->
    {stingray_ast:type_ref(Name, {L, C}), Rest};

%% Parenthesized expression
parse_expr_primary([{lparen, _, _, _} | Rest]) ->
    {Expr, Rest1} = parse_expression(Rest),
    {_, Rest2} = expect(rparen, Rest1),
    {Expr, Rest2};

%% List literal (including empty list [])
parse_expr_primary([{lbracket, _, L, C}, {rbracket, _, _, _} | Rest]) ->
    {stingray_ast:list_literal([], {L, C}), Rest};
parse_expr_primary([{lbracket, _, L, C} | Rest]) ->
    {Elements, Rest1} = parse_list_elements(Rest, []),
    {stingray_ast:list_literal(Elements, {L, C}), Rest1};

%% Set literal
parse_expr_primary([{langle, _, L, C} | Rest]) ->
    {Elements, Rest1} = parse_list_elements(Rest, []),
    {stingray_ast:set_literal(Elements, {L, C}), Rest1};

parse_expr_primary([]) ->
    throw({error, {0, 0, unexpected_eof}});
parse_expr_primary([{_, _, L, C} | _]) ->
    throw({error, {L, C, unexpected_token}}).

%% ============================================================================
%% Argument/element lists
%% ============================================================================

parse_args([{rparen, _, _, _} | _] = Tokens) ->
    {[], Tokens};
parse_args(Tokens) ->
    parse_args_loop(Tokens, []).

parse_args_loop([{rparen, _, _, _} | _] = Tokens, Acc) ->
    {lists:reverse(Acc), Tokens};
parse_args_loop([{comma, _, _, _}, {rparen, _, _, _} | _], Acc) ->
    %% Trailing comma before ) — skip comma and return with rparen
    {lists:reverse(Acc), [{rparen, 0, 0, 0}]};
parse_args_loop(Tokens, Acc) ->
    {Expr, Rest} = parse_expression(Tokens),
    case Rest of
        [{comma, _, _, _} | Rest1] -> parse_args_loop(Rest1, [Expr | Acc]);
        _ -> {lists:reverse([Expr | Acc]), Rest}
    end.

parse_list_elements([{rbracket, _, _, _} | Rest], Acc) ->
    {lists:reverse(Acc), Rest};
parse_list_elements([{rangle, _, _, _} | Rest], Acc) ->
    {lists:reverse(Acc), Rest};
parse_list_elements([], _Acc) ->
    throw({error, {0, 0, unterminated_list}});
parse_list_elements(Tokens, Acc) ->
    {Expr, Rest} = parse_expression(Tokens),
    case Rest of
        [{comma, _, _, _} | Rest1] -> parse_list_elements(Rest1, [Expr | Acc]);
        _ -> parse_list_elements(Rest, [Expr | Acc])
    end.

%% ============================================================================
%% Helpers
%% ============================================================================

expect(Type, [{Type, Value, _, _} | Rest]) -> {Value, Rest};
expect(Type, [{_, _, L, C} | _]) -> throw({error, {L, C, {expected_token, Type}}});
expect(_, []) -> throw({error, {0, 0, unexpected_eof}}).

expect_name([{identifier, Value, _, _} | Rest]) -> {Value, Rest};
expect_name([{type_identifier, Value, _, _} | Rest]) -> {Value, Rest};
expect_name([{_, _, L, C} | _]) -> throw({error, {L, C, {expected_token, name}}});
expect_name([]) -> throw({error, {0, 0, unexpected_eof}}).

token_loc({_, _, _, L, C}) -> {L, C};
token_loc({_, _, L, C}) -> {L, C};
token_loc(_) -> {0, 0}.