-module(run_tests).
-export([run/0]).

run() ->
    code:add_patha("src"),
    Dir = "C:/Users/User/Desktop/StingrayCompiler",
    Tests = [
        {"hello_world", Dir ++ "/examples/hello_world.sr", hello_world},
        {"struct_and_enum", Dir ++ "/examples/struct_and_enum.sr", struct_enum},
        {"if_else_test", Dir ++ "/examples/if_else_test.sr", if_else},
        {"full_example", Dir ++ "/examples/full_example.sr", full_example},
        {"enum_test", Dir ++ "/examples/enum_test.sr", enum_test},
        {"struct_test", Dir ++ "/examples/struct_test.sr", struct_test},
        {"list_test", Dir ++ "/examples/list_test.sr", list_test},
        {"while_test", Dir ++ "/examples/while_test.sr", while_test},
        {"all_features", Dir ++ "/examples/all_features_test.sr", all_features}
    ],
    io:format("~n========================================~n"),
    io:format("         STINGRAY COMPILER TESTS~n"),
    io:format("========================================~n~n"),
    Results = [run_one(N, F, M) || {N, F, M} <- Tests],
    Passed = lists:foldl(fun({_,S,_}, A) -> case S of ok -> A+1; _ -> A end end, 0, Results),
    Total = length(Results),
    io:format("~n========================================~n"),
    io:format("  Total: ~p  Passed: ~p  Failed: ~p~n", [Total, Passed, Total - Passed]),
    io:format("========================================~n").

run_one(Name, File, Mod) ->
    io:format("--- ~s ---~n", [Name]),
    case file:read_file(File) of
        {ok, Bin} ->
            case catch run_one_inner(Bin, File, Mod) of
                ok -> {Name, ok, ""};
                {'EXIT', Reason} -> {Name, error, lists:flatten(io_lib:format("~p", [Reason]))};
                Other -> {Name, error, lists:flatten(io_lib:format("~p", [Other]))}
            end;
        {error, Reason} ->
            io:format("  File error: ~p~n", [Reason]),
            {Name, error, "file not found"}
    end.

run_one_inner(Bin, File, Mod) ->
    Tokens = stingray_lexer:tokenize(Bin),
    {ok, AST} = stingray_parser:parse(Tokens),
    {M, B} = case stingray_codegen:compile(AST, [{module, Mod}]) of
        {ok, M0, B0} -> {M0, B0};
        {ok, M0, B0, _} -> {M0, B0}
    end,
    {module, M} = code:load_binary(M, File, B),
    M:main(),
    timer:sleep(50),
    ok.
