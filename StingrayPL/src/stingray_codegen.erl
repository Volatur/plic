-module(stingray_codegen).
-export([compile/1, compile/2]).

%% Compile AST to .beam binary
compile(AST) ->
    compile(AST, [{module, stingray_output}]).

compile({program, TopLevel}, Options) ->
    ModuleName = proplists:get_value(module, Options, stingray_output),
    put(while_funcs, []),
    put(structs, #{}),
    put(enums, #{}),
    put(var_types, #{}),  %% #{VarName => {struct, TypeName} | {enum, EnumName}}

    %% First pass: collect struct and enum metadata
    collect_metadata(TopLevel),

    %% Generate alias definitions from use directives
    AliasDefs = lists:filtermap(fun
        ({use_directive, Mod, Alias, L, _}) ->
            {true, {match, L, {var, L, safe_name(Alias)}, {atom, L, safe_name(Mod)}}};
        (_) -> false
    end, TopLevel),

    %% Generate enum bindings
    EnumDefs = lists:filtermap(fun
        ({enum_decl, Name, Values, L, _}) ->
            EnumTuple = {tuple, L, [{atom, L, safe_name(V)} || V <- Values]},
            {true, {match, L, {var, L, safe_name(Name)}, EnumTuple}};
        (_) -> false
    end, TopLevel),

    %% Compile functions (skip enums, structs, type aliases, use directives, forward decls)
    Functions = lists:filtermap(fun compile_top_level/1, TopLevel),

    WhileFuncs = case get(while_funcs) of undefined -> []; WFs -> lists:reverse(WFs) end,

    %% Find all main functions and prepend alias + enum defs
    {MainFuncs, OtherFuncs} = lists:partition(fun
        ({function, _, main, 0, _}) -> true;
        (_) -> false
    end, Functions),
    FinalMainFuncs = lists:map(fun
        ({function, L, main, 0, [{clause, LL, Params, Guards, Body}]}) ->
            {function, L, main, 0, [{clause, LL, Params, Guards, AliasDefs ++ EnumDefs ++ Body}]}
    end, MainFuncs),

    ModuleAttr = {attribute, 1, module, ModuleName},
    ExportAttr = {attribute, 2, export, [{main, 0}]},
    AllForms = [ModuleAttr, ExportAttr] ++ WhileFuncs ++ OtherFuncs ++ FinalMainFuncs,

    case compile:forms(AllForms, [return_errors, return_warnings]) of
        {ok, Module, BeamBinary, _Warnings} -> {ok, Module, BeamBinary};
        {ok, Module, BeamBinary} -> {ok, Module, BeamBinary};
        {error, Errors, _} -> {error, Errors}
    end.

%% ============================================================================
%% Metadata collection (structs, enums)
%% ============================================================================

collect_metadata([]) -> ok;
collect_metadata([{enum_decl, Name, Values, _, _} | Rest]) ->
    EnumMap = get(enums),
    put(enums, EnumMap#{safe_name(Name) => [safe_name(V) || V <- Values]}),
    collect_metadata(Rest);
collect_metadata([{struct_decl, Name, Fields, _, _, _} | Rest]) ->
    FieldNames = [safe_name(FName) || {field, FName, _, _, _} <- Fields],
    StructMap = get(structs),
    put(structs, StructMap#{safe_name(Name) => FieldNames}),
    collect_metadata(Rest);
collect_metadata([{fun_decl, _, Params, Body, _, _} | Rest]) ->
    %% Also collect structs/enums inside struct method bodies if any
    collect_metadata(Body),
    collect_metadata(Rest);
collect_metadata([{type_alias, _, _, _, _} | Rest]) ->
    collect_metadata(Rest);
collect_metadata([{use_directive, _, _, _, _} | Rest]) ->
    collect_metadata(Rest);
collect_metadata([{assignment, _, _, _, _} | Rest]) ->
    collect_metadata(Rest);
collect_metadata([{sideway_directive, _, _, _} | Rest]) ->
    collect_metadata(Rest);
collect_metadata([{flow_directive, _, _, _, _} | Rest]) ->
    collect_metadata(Rest);
collect_metadata([{var_decl, _, _, _, _, _} | Rest]) ->
    collect_metadata(Rest);
collect_metadata([{while_stmt, _, Body, _, _} | Rest]) ->
    collect_metadata(Body),
    collect_metadata(Rest);
collect_metadata([{if_stmt, _, Then, Else, _, _} | Rest]) ->
    collect_metadata(Then),
    collect_metadata(Else),
    collect_metadata(Rest);
collect_metadata([{expr_stmt, _, _, _} | Rest]) ->
    collect_metadata(Rest);
collect_metadata([{return_stmt, _, _, _} | Rest]) ->
    collect_metadata(Rest);
collect_metadata([{block, Stmts, _, _} | Rest]) ->
    collect_metadata(Stmts),
    collect_metadata(Rest);
collect_metadata([{_, _, _, _, _} | Rest]) ->
    collect_metadata(Rest);
collect_metadata([{_, _, _, _} | Rest]) ->
    collect_metadata(Rest);
collect_metadata(_) -> ok.

%% ============================================================================
%% Top-level compilation
%% ============================================================================

compile_top_level({fun_decl, Name, Params, Body, L, _C}) ->
    %% Skip forward declarations (empty params and empty body)
    case {Params, Body} of
        {[], []} -> false;
        _ ->
            Arity = length(Params),
            ErlName = safe_name(Name),
            %% Track parameter types in var_types
            lists:foreach(fun(Param) ->
                case Param of
                    {param, PName, {type_basic, TypeName, _, _}, _, _} ->
                        put(var_types, maps:put(safe_name(PName), {struct, safe_name(TypeName)}, get(var_types)));
                    _ -> ok
                end
            end, Params),
            ErlParams = [{var, L, safe_name(PName)} || {param, PName, _, _, _} <- Params],
            ErlBody = compile_block(Body),
            Clause = {clause, L, ErlParams, [], ErlBody},
            {true, {function, L, ErlName, Arity, [Clause]}}
    end;
compile_top_level({enum_decl, _, _, _, _}) -> false;
compile_top_level({struct_decl, _, _, _, _, _}) -> false;
compile_top_level({type_alias, _, _, _, _}) -> false;
compile_top_level({use_directive, _, _, _, _}) -> false;
compile_top_level({sideway_directive, _, _, _}) -> false;
compile_top_level({flow_directive, _, _, _, _}) -> false;
compile_top_level(_) -> false.

%% ============================================================================
%% Block compilation — detects var_decl + while merge pattern
%% ============================================================================

compile_block(Stmts) ->
    compile_block(Stmts, []).

compile_block([], Acc) -> lists:reverse(Acc);
compile_block([{while_stmt, Cond, Body, WL, WC} | Rest], Acc) ->
    %% Check if previous statement is var_decl for a variable used in while body
    case Acc of
        [{match, ML, {var, MV, VarAtom}, InitExpr} | PrevAcc] ->
            case is_var_mutated_in_body(VarAtom, Body) of
                true ->
                    %% Merge: absorb the init into the while loop
                    %% Rename all references to VarAtom inside while body to a param name
                    ParamAtom = list_to_atom("__w_" ++ atom_to_list(VarAtom)),
                    Renames = #{VarAtom => ParamAtom},
                    RenamedCond = ast_rename(Cond, Renames),
                    %% Split body: side effects + last assignment to loop var
                    {SideEffects, LastAssignRHS} = split_last_assignment(VarAtom, Body),
                    RenamedSideEffects = [ast_rename(S, Renames) || S <- SideEffects],
                    ErlCond = compile_expr(RenamedCond),
                    ErlSideEffects = compile_block_recur(RenamedSideEffects),
                    %% The last assignment's RHS becomes the recursive call arg
                    ErlNewVal = compile_expr(ast_rename(LastAssignRHS, Renames)),
                    ParamVar = {var, WL, ParamAtom},
                    RecurseExpr = {call, WL, {var, WL, 'Loop'}, [ErlNewVal]},
                    NamedFun = {named_fun, WL, 'Loop', [
                        {clause, WL, [ParamVar], [], [
                            {'case', WL, ErlCond, [
                                {clause, WL, [{atom, WL, true}], [],
                                    ErlSideEffects ++ [RecurseExpr]},
                                {clause, WL, [{atom, WL, false}], [], [ParamVar]}
                            ]}
                        ]}
                    ]},
                    compile_block(Rest, [{match, ML, {var, MV, VarAtom}, {call, WL, NamedFun, [InitExpr]}} | PrevAcc]);
                false ->
                    WhileExpr = compile_while_inline(Cond, Body, WL, WC),
                    compile_block(Rest, [WhileExpr | Acc])
            end;
        _ ->
            WhileExpr = compile_while_inline(Cond, Body, WL, WC),
            compile_block(Rest, [WhileExpr | Acc])
    end;
compile_block([Stmt | Rest], Acc) ->
    case compile_statement(Stmt) of
        {true, Expr} -> compile_block(Rest, [Expr | Acc]);
        false -> compile_block(Rest, Acc)
    end.

compile_block_recur(Stmts) ->
    lists:filtermap(fun compile_statement/1, Stmts).

%% Compile while as simple inline named fun (for cases without var_decl merge)
compile_while_inline(Cond, Body, L, _C) ->
    ErlCond = compile_expr(Cond),
    ErlBody = compile_block_recur(Body),
    %% Find free vars from condition and body
    FreeVars = collect_free_vars_ast([Cond | Body]),
    %% Create params and rename mapping
    ParamNames = [{V, list_to_atom("__w_" ++ atom_to_list(V) ++ "_" ++ integer_to_list(
        erlang:unique_integer([positive])))} || V <- FreeVars],
    RenameMap = maps:from_list(ParamNames),
    %% Rename condition
    RenamedCond = ast_rename(Cond, RenameMap),
    ErlCond2 = compile_expr(RenamedCond),
    %% Rename body and compile
    RenamedBody = [ast_rename(S, RenameMap) || S <- Body],
    ErlBody2 = compile_block_recur(RenamedBody),
    %% Build params and init values
    Params = [{var, L, NewName} || {_Old, NewName} <- ParamNames],
    InitVals = [compile_expr({identifier, Old, L, 0}) || {Old, _New} <- ParamNames],
    %% Build body
    BodyExprs = case ErlBody2 of [] -> [{nil, L}]; _ -> ErlBody2 end,
    %% Recursive call uses renamed param names, not original names
    RecurseArgs = [{var, L, NewName} || {_Old, NewName} <- ParamNames],
    NamedFun = {named_fun, L, 'Loop', [
        {clause, L, Params, [], [
            {'case', L, ErlCond2, [
                {clause, L, [{atom, L, true}], [], BodyExprs ++ [{call, L, {var, L, 'Loop'}, RecurseArgs}]},
                {clause, L, [{atom, L, false}], [], [{nil, L}]}
            ]}
        ]}
    ]},
    case InitVals of
        [] -> {call, L, NamedFun, []};
        _ -> {call, L, NamedFun, InitVals}
    end.

%% ============================================================================
%% Statement compilation
%% ============================================================================

compile_statement({var_decl, Name, _Type, Init, L, _C}) ->
    ErlInit = compile_expr(Init),
    track_var_type(Name, Init),
    {true, {match, L, {var, L, safe_name(Name)}, ErlInit}};

compile_statement({assignment, Target, Value, L, _C}) ->
    ErlTarget = case Target of
        {identifier, N, _, _} ->
            track_var_type(N, Value),
            {var, L, safe_name(N)};
        _ -> {var, L, '_'}
    end,
    ErlValue = compile_expr(Value),
    {true, {match, L, ErlTarget, ErlValue}};

compile_statement({while_stmt, Cond, Body, L, _C}) ->
    ErlCond = compile_expr(Cond),
    ErlBody = compile_block(Body),
    FuncName = list_to_atom("while_" ++ integer_to_list(erlang:unique_integer([positive]))),
    WhileFunc = {function, L, FuncName, 0, [
        {clause, L, [], [], [
            {'case', L, ErlCond, [
                {clause, L, [{atom, L, true}], [], ErlBody ++ [{call, L, {atom, L, FuncName}, []}]},
                {clause, L, [{atom, L, false}], [], [{nil, L}]}
            ]}
        ]}
    ]},
    store_while_func(WhileFunc),
    {true, {call, L, {atom, L, FuncName}, []}};

compile_statement({if_stmt, Cond, ThenBody, ElseBody, L, _C}) ->
    ErlCond = compile_expr(Cond),
    ErlThen = compile_block(ThenBody),
    ErlElse = compile_block(ElseBody),
    ThenExprs = case ErlThen of [] -> [{nil, L}]; _ -> ErlThen end,
    ElseExprs = case ErlElse of [] -> [{nil, L}]; _ -> ErlElse end,
    {true, {'case', L, ErlCond, [
        {clause, L, [{atom, L, true}], [], ThenExprs},
        {clause, L, [{atom, L, false}], [], ElseExprs}
    ]}};

compile_statement({expr_stmt, Expr, _L, _C}) ->
    {true, compile_expr(Expr)};

compile_statement({return_stmt, Expr, L, _C}) ->
    {true, compile_expr(Expr)};

%% #sideway# expr — fire and forget: spawn(fun() -> expr end)
compile_statement({sideway_directive, Expr, L, _C}) ->
    ErlExpr = compile_expr(Expr),
    SpawnFun = {'fun', L, {clauses, [{clause, L, [], [], [ErlExpr]}]}},
    {true, {call, L, {atom, L, spawn}, [SpawnFun]}};

%% #flow:N# expr — spawn N parallel copies, wait for all
compile_statement({flow_directive, Count, Expr, L, _C}) ->
    ErlExpr = compile_expr(Expr),
    WorkerFun = {'fun', L, {clauses, [{clause, L, [], [], [ErlExpr]}]}},
    CountExpr = {integer, L, Count},
    {true, {call, L, {remote, L, {atom, L, stingray_runtime}, {atom, L, flow_run}},
        [CountExpr, WorkerFun]}};
compile_statement({enum_decl, _, _, _, _}) -> false;
compile_statement({struct_decl, _, _, _, _, _}) -> false;
compile_statement({type_alias, _, _, _, _}) -> false;
compile_statement({use_directive, _, _, _, _}) -> false;
compile_statement(_) -> false.

%% ============================================================================
%% Expression compilation
%% ============================================================================

compile_expr({int_literal, Value, L, _}) -> {integer, L, Value};
compile_expr({float_literal, Value, L, _}) -> {float, L, Value};
compile_expr({string_literal, Value, L, _}) -> {string, L, binary_to_list(Value)};
compile_expr({char_literal, Value, L, _}) -> {integer, L, Value};
compile_expr({bool_literal, true, L, _}) -> {atom, L, true};
compile_expr({bool_literal, false, L, _}) -> {atom, L, false};

compile_expr({identifier, Name, L, _}) -> {var, L, safe_name(Name)};
compile_expr({type_ref, Name, L, _}) -> {atom, L, safe_name(Name)};

compile_expr({binary_op, Op, Left, Right, L, _}) ->
    ErlOp = case Op of
        'and' -> 'andalso';
        'or' -> 'orelse';
        '+' -> str_plus;
        Other -> Other
    end,
    case ErlOp of
        str_plus ->
            %% String-aware +: uses stingray_runtime:str_append/2
            {call, L, {remote, L, {atom, L, stingray_runtime}, {atom, L, str_append}},
                [compile_expr(Left), compile_expr(Right)]};
        _ ->
            {op, L, ErlOp, compile_expr(Left), compile_expr(Right)}
    end;
compile_expr({unary_op, '++post', Operand, L, _}) ->
    %% i++ → i + 1 (returns new value)
    {op, L, '+', compile_expr(Operand), {integer, L, 1}};
compile_expr({unary_op, Op, Operand, L, _}) ->
    {op, L, Op, compile_expr(Operand)};

%% Module-qualified call or method call: io.write(...) or list.push(...)
compile_expr({call, {member_access, {identifier, Mod, _, _}, Func, _, _}, Args, L, _}) ->
    ModAtom = safe_name(Mod),
    FuncAtom = safe_name(Func),
    case {ModAtom, FuncAtom} of
        {io, write} ->
            compile_io_write(Args, L);
        {_, push} when length(Args) =:= 1 ->
            %% list.push(item) → list ++ [item]
            ObjectExpr = compile_expr({identifier, Mod, L, 0}),
            ItemExpr = compile_expr(hd(Args)),
            {op, L, '++', ObjectExpr, {cons, L, ItemExpr, {nil, L}}};
        {_, pop} when Args =:= [] ->
            %% list.pop() → lists:last(list)
            ObjectExpr = compile_expr({identifier, Mod, L, 0}),
            {call, L, {remote, L, {atom, L, lists}, {atom, L, last}}, [ObjectExpr]};
        {_, length} when Args =:= [] ->
            %% list.length → erlang:length(list)
            ObjectExpr = compile_expr({identifier, Mod, L, 0}),
            {call, L, {remote, L, {atom, L, erlang}, {atom, L, length}}, [ObjectExpr]};
        _ ->
            ErlArgs = [compile_expr(A) || A <- Args],
            {call, L, {remote, L, {atom, L, ModAtom}, {atom, L, FuncAtom}}, ErlArgs}
    end;
%% Direct function call: foo(args)
compile_expr({call, {identifier, Name, _, _}, Args, L, _}) ->
    ErlArgs = [compile_expr(A) || A <- Args],
    {call, L, {atom, L, safe_name(Name)}, ErlArgs};
%% Fallback call
compile_expr({call, Callee, Args, L, _}) ->
    ErlArgs = [compile_expr(A) || A <- Args],
    ErlCallee = compile_expr(Callee),
    {call, L, ErlCallee, ErlArgs};

%% Member access: object.field
%% Resolve struct field index from metadata, or list.length, or enum values
compile_expr({member_access, Object, Field, L, _}) ->
    ObjectExpr = compile_expr(Object),
    FieldAtom = safe_name(Field),
    case FieldAtom of
        length ->
            %% list.length → erlang:length(list)
            {call, L, {remote, L, {atom, L, erlang}, {atom, L, length}}, [ObjectExpr]};
        _ ->
            case resolve_field_index(Object, FieldAtom) of
                {ok, Index} ->
                    {call, L, {atom, L, element}, [{integer, L, Index}, ObjectExpr]};
                error ->
                    {call, L, {atom, L, element}, [{integer, L, 1}, ObjectExpr]}
            end
    end;

%% new TypeName(args)
compile_expr({new_expr, TypeName, Args, L, _}) ->
    Elements = [{atom, L, safe_name(TypeName)}] ++ [compile_expr(A) || A <- Args],
    {tuple, L, Elements};

compile_expr({list_literal, Elements, L, _}) ->
    lists:foldr(fun(E, Acc) -> {cons, L, compile_expr(E), Acc} end, {nil, L}, Elements);

compile_expr({set_literal, Elements, L, _}) ->
    ListExpr = compile_expr({list_literal, Elements, L, 0}),
    {call, L, {remote, L, {atom, L, ordsets}, {atom, L, from_list}}, [ListExpr]};

%% list[index] — lists:nth(Index+1, List)
compile_expr({list_access, List, Index, L, _}) ->
    ListExpr = compile_expr(List),
    IndexExpr = compile_expr(Index),
    OneBased = {op, L, '+', IndexExpr, {integer, L, 1}},
    {call, L, {remote, L, {atom, L, lists}, {atom, L, nth}}, [OneBased, ListExpr]};

compile_expr(_) -> {atom, 1, undefined}.

%% ============================================================================
%% io.write compilation
%% ============================================================================

compile_io_write([{string_literal, Val, _, _}], L) ->
    %% io.write("text") -> io:format("~ts", [["text"]])
    StringList = {cons, L, {string, L, binary_to_list(Val)}, {nil, L}},
    {call, L, {remote, L, {atom, L, io}, {atom, L, format}},
        [{string, L, "~ts"}, StringList]};
compile_io_write(Args, L) ->
    %% io.write(X) -> io:format("~p~n", [X])  (no newline)
    ErlArgs = [compile_expr(A) || A <- Args],
    ArgList = lists:foldr(fun(A, Acc) -> {cons, L, A, Acc} end, {nil, L}, ErlArgs),
    {call, L, {remote, L, {atom, L, io}, {atom, L, format}},
        [{string, L, "~p"}, ArgList]}.

%% ============================================================================
%% Struct/enum field resolution
%% ============================================================================

%% Track that a variable holds a struct or enum type
track_var_type(VarName, {new_expr, TypeName, _, _, _}) ->
    VarTypes = get(var_types),
    put(var_types, VarTypes#{safe_name(VarName) => {struct, safe_name(TypeName)}});
track_var_type(VarName, {member_access, {identifier, ObjName, _, _}, Field, _, _}) ->
    VarTypes = get(var_types),
    FieldAtom = safe_name(Field),
    %% Check if ObjName is a known enum → this is enum member access
    EnumMap = get(enums),
    ObjAtom = safe_name(ObjName),
    case maps:find(ObjAtom, EnumMap) of
        {ok, Values} ->
            case lists:member(FieldAtom, Values) of
                true ->
                    put(var_types, VarTypes#{safe_name(VarName) => {enum_value, ObjAtom, FieldAtom}});
                false -> ok
            end;
        error -> ok
    end;
track_var_type(_, _) -> ok.

%% Resolve field index for member access
resolve_field_index(ObjectExpr, FieldAtom) ->
    case ObjectExpr of
        {identifier, Name, _, _} ->
            NameAtom = safe_name(Name),
            StructMap = get(structs),
            %% First: check if Name is a struct type directly (e.g., Point.x)
            case maps:find(NameAtom, StructMap) of
                {ok, FieldNames} ->
                    {ok, find_field_pos(FieldNames, FieldAtom, 2)};
                error ->
                    %% Second: check variable type tracking (e.g., p.x where p is Point)
                    VarTypes = get(var_types),
                    case maps:find(NameAtom, VarTypes) of
                        {ok, {struct, StructName}} ->
                            case maps:find(StructName, StructMap) of
                                {ok, FieldNames2} ->
                                    {ok, find_field_pos(FieldNames2, FieldAtom, 2)};
                                error -> error
                            end;
                        {ok, {enum_value, EnumName, Value}} ->
                            %% This variable holds a single enum value — can't do field access
                            error;
                        error ->
                            %% Third: check if Name is an enum and Field is a value
                            resolve_enum_access(NameAtom, FieldAtom)
                    end
            end;
        _ -> error
    end.

%% Enum member access: color.RED → element(N, color)
resolve_enum_access(EnumAtom, FieldAtom) ->
    EnumMap = get(enums),
    case maps:find(EnumAtom, EnumMap) of
        {ok, Values} ->
            case find_field_pos(Values, FieldAtom, 1) of
                Pos when Pos > 0 -> {ok, Pos};
                _ -> error
            end;
        error -> error
    end.

find_field_pos([], _Field, _Pos) -> 1;  %% fallback to element(1) (tag)
find_field_pos([Field | _], Field, Pos) -> Pos;
find_field_pos([_ | Rest], Field, Pos) -> find_field_pos(Rest, Field, Pos + 1).

%% ============================================================================
%% Helpers
%% ============================================================================

safe_name(Name) when is_binary(Name) -> binary_to_atom(Name, utf8);
safe_name(Name) when is_atom(Name) -> Name;
safe_name(_) -> '__anonymous'.

store_while_func(F) ->
    case get(while_funcs) of
        undefined -> put(while_funcs, [F]);
        L -> put(while_funcs, [F | L])
    end.

%% ============================================================================
%% While loop helpers
%% ============================================================================

%% Check if a variable is assigned (mutated) inside the while body
is_var_mutated_in_body(VarAtom, Body) ->
    lists:any(fun(S) -> is_var_mutated_in_stmt(VarAtom, S) end, Body).

is_var_mutated_in_stmt(VarAtom, {assignment, {identifier, N, _, _}, _, _, _}) ->
    safe_name(N) =:= VarAtom;
is_var_mutated_in_stmt(VarAtom, {var_decl, N, _, _, _, _}) ->
    safe_name(N) =:= VarAtom;
is_var_mutated_in_stmt(VarAtom, {while_stmt, _, Body, _, _}) ->
    is_var_mutated_in_body(VarAtom, Body);
is_var_mutated_in_stmt(VarAtom, {if_stmt, _, Then, Else, _, _}) ->
    is_var_mutated_in_body(VarAtom, Then) orelse is_var_mutated_in_body(VarAtom, Else);
is_var_mutated_in_stmt(_, _) -> false.

%% Collect free variables from an AST node (identifiers that appear)
collect_free_vars_ast(Nodes) ->
    sets:to_list(lists:foldl(fun collect_free_vars_node/2, sets:new(), Nodes)).

collect_free_vars_node({identifier, Name, _, _}, Acc) ->
    sets:add_element(safe_name(Name), Acc);
collect_free_vars_node({binary_op, _, Left, Right, _, _}, Acc) ->
    collect_free_vars_node(Left, collect_free_vars_node(Right, Acc));
collect_free_vars_node({unary_op, _, Operand, _, _}, Acc) ->
    collect_free_vars_node(Operand, Acc);
collect_free_vars_node({call, Callee, Args, _, _}, Acc) ->
    Acc1 = collect_free_vars_node(Callee, Acc),
    lists:foldl(fun collect_free_vars_node/2, Acc1, Args);
collect_free_vars_node({member_access, Obj, _, _, _}, Acc) ->
    collect_free_vars_node(Obj, Acc);
collect_free_vars_node({new_expr, _, Args, _, _}, Acc) ->
    lists:foldl(fun collect_free_vars_node/2, Acc, Args);
collect_free_vars_node({list_literal, Elems, _, _}, Acc) ->
    lists:foldl(fun collect_free_vars_node/2, Acc, Elems);
collect_free_vars_node({assignment, Target, Value, _, _}, Acc) ->
    collect_free_vars_node(Value, collect_free_vars_node(Target, Acc));
collect_free_vars_node({var_decl, _, _, Init, _, _}, Acc) ->
    collect_free_vars_node(Init, Acc);
collect_free_vars_node({while_stmt, Cond, Body, _, _}, Acc) ->
    lists:foldl(fun collect_free_vars_node/2, collect_free_vars_node(Cond, Acc), Body);
collect_free_vars_node({if_stmt, Cond, Then, Else, _, _}, Acc) ->
    Acc1 = collect_free_vars_node(Cond, Acc),
    Acc2 = lists:foldl(fun collect_free_vars_node/2, Acc1, Then),
    lists:foldl(fun collect_free_vars_node/2, Acc2, Else);
collect_free_vars_node(_, Acc) -> Acc.

%% Rename identifiers in AST
ast_rename(Node, Renames) when is_map(Renames) -> ast_rename_node(Node, Renames).

ast_rename_node({identifier, Name, L, C}, Renames) ->
    NewName = maps:get(safe_name(Name), Renames, safe_name(Name)),
    {identifier, NewName, L, C};
ast_rename_node({binary_op, Op, Left, Right, L, C}, Renames) ->
    {binary_op, Op, ast_rename_node(Left, Renames), ast_rename_node(Right, Renames), L, C};
ast_rename_node({unary_op, Op, Operand, L, C}, Renames) ->
    {unary_op, Op, ast_rename_node(Operand, Renames), L, C};
ast_rename_node({call, Callee, Args, L, C}, Renames) ->
    {call, ast_rename_node(Callee, Renames), [ast_rename_node(A, Renames) || A <- Args], L, C};
ast_rename_node({member_access, Obj, Field, L, C}, Renames) ->
    {member_access, ast_rename_node(Obj, Renames), Field, L, C};
ast_rename_node({new_expr, Name, Args, L, C}, Renames) ->
    {new_expr, Name, [ast_rename_node(A, Renames) || A <- Args], L, C};
ast_rename_node({list_literal, Elems, L, C}, Renames) ->
    {list_literal, [ast_rename_node(E, Renames) || E <- Elems], L, C};
ast_rename_node({assignment, Target, Value, L, C}, Renames) ->
    {assignment, ast_rename_node(Target, Renames), ast_rename_node(Value, Renames), L, C};
ast_rename_node({var_decl, Name, Type, Init, L, C}, Renames) ->
    NewName = maps:get(safe_name(Name), Renames, safe_name(Name)),
    {var_decl, NewName, Type, ast_rename_node(Init, Renames), L, C};
ast_rename_node({while_stmt, Cond, Body, L, C}, Renames) ->
    {while_stmt, ast_rename_node(Cond, Renames), [ast_rename_node(S, Renames) || S <- Body], L, C};
ast_rename_node({if_stmt, Cond, Then, Else, L, C}, Renames) ->
    {if_stmt, ast_rename_node(Cond, Renames), [ast_rename_node(S, Renames) || S <- Then], [ast_rename_node(S, Renames) || S <- Else], L, C};
ast_rename_node({return_stmt, Expr, L, C}, Renames) ->
    {return_stmt, ast_rename_node(Expr, Renames), L, C};
ast_rename_node({expr_stmt, Expr, L, C}, Renames) ->
    {expr_stmt, ast_rename_node(Expr, Renames), L, C};
ast_rename_node(Node, _Renames) -> Node.

%% Split body into {SideEffects, LastAssignRHS}
%% The last statement that assigns to VarAtom is extracted; its RHS is returned separately
split_last_assignment(_VarAtom, []) -> {[], {int_literal, 0, 0, 0}};
split_last_assignment(VarAtom, StmtList) ->
    split_last_assignment(VarAtom, StmtList, []).

split_last_assignment(VarAtom, [Last], Acc) ->
    case is_assignment_to(Last, VarAtom) of
        {true, RHS} -> {lists:reverse(Acc), RHS};
        false -> {lists:reverse(Acc ++ [Last]), {int_literal, 0, 0, 0}}
    end;
split_last_assignment(VarAtom, [Stmt | Rest], Acc) ->
    split_last_assignment(VarAtom, Rest, [Stmt | Acc]).

is_assignment_to({assignment, {identifier, N, _, _}, Value, _, _}, VarAtom) ->
    case safe_name(N) =:= VarAtom of
        true -> {true, Value};
        false -> false
    end;
is_assignment_to({var_decl, N, _, Init, _, _}, VarAtom) ->
    case safe_name(N) =:= VarAtom of
        true -> {true, Init};
        false -> false
    end;
is_assignment_to(_, _) -> false.
