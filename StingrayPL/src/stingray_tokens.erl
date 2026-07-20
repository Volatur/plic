-module(stingray_tokens).
-export([keyword_map/0, token_type/1, token_value/1, token_line/1, token_col/1]).

-type token() :: {atom(), binary() | number(), pos_integer(), pos_integer()}.
-export_type([token/0]).

-spec keyword_map() -> #{binary() => atom()}.
keyword_map() ->
    #{
        <<"fun">>     => keyword_fun,
        <<"enum">>    => keyword_enum,
        <<"type">>    => keyword_type,
        <<"struct">>  => keyword_struct,
        <<"while">>   => keyword_while,
        <<"as">>      => keyword_as,
        <<"if">>      => keyword_if,
        <<"else">>    => keyword_else,
        <<"return">>  => keyword_return,
        <<"new">>     => keyword_new,
        <<"not">>     => keyword_not,
        <<"true">>    => keyword_true,
        <<"false">>   => keyword_false
    }.

-spec token_type(token()) -> atom().
token_type({Type, _, _, _}) -> Type.

-spec token_value(token()) -> binary() | number().
token_value({_, Value, _, _}) -> Value.

-spec token_line(token()) -> pos_integer().
token_line({_, _, Line, _}) -> Line.

-spec token_col(token()) -> pos_integer().
token_col({_, _, _, Col}) -> Col.