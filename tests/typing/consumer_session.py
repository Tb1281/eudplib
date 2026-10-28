# pyright: strict

"""Consumer fixture checked by Pyright against an installed eudkit wheel.

This file is intentionally not executed. It verifies that the wheel is marked
with ``py.typed`` and that the ambient ``session`` proxy is exposed to type
checkers as a :class:`eudkit.Session`.
"""

from typing_extensions import assert_type, reveal_type

from eudkit import MapContext, Session, Var, session

assert_type(session, Session)
assert_type(session.map_context, MapContext)
assert_type(session.var(), Var)
assert_type(session.var(init=10, name="typed_var"), Var)

assert_type(session.map_context.location_id("Anywhere"), int)
assert_type(session.map_context.unit_id("Terran Marine"), int)
assert_type(session.map_context.switch_id("Switch 1"), int)
assert_type(session.map_context.get_or_insert_string("typed"), int)
assert_type(session.map_context.classic_trigger_count, int)
assert_type(session.map_context.locations_count, int)
assert_type(session.map_context.placed_units_count, int)

reveal_type(session)
reveal_type(session.map_context)
reveal_type(session.var())
