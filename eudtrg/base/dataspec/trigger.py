"""
This file defines class Trigger, Condition, Actions. Trigger class represents
triggers with next pointer. Each class is addressable. This file also defines
PushTriggerScope and PopTriggerScope function for trigger scoping.
"""

from . import addressable
from ..payload import depgraph
from .expr import IsValidExpr
from ..utils.utils import FlattenList

_last_trigger_stack = [None]
_token_stack = []


"""
Creates trigger scope. Scope semantically links related triggers.
ex)
  PushTriggerScope()
  # Triggers consisting function f_add
  PopTriggerScope()

Triggers in the same scope are eligible for auto link.
"""
def PushTriggerScope():
	_last_trigger_stack.append(None)
	class Token:
		pass
	_token_stack.append(Token)
	return Token


def PopTriggerScope(token = None):
	if token:
		assert _token_stack[-1] == token, "Trigger scoping error"
	_token_stack.pop()
	_last_trigger_stack.pop()

"""
Trigger class. This class is immutable.
"""
class Trigger(addressable.Addressable):
	"""
	Constructor for trigger
	 - nextptr : Addr to trigger to be executed after this trigger.
	 - conditions : Condition consisting this trigger. Must be less than 16
	 - actions : Action consisting this trigger. Must be less than 64
	ref) GetNextPtrAddr, GetConditionAddr, GetActionAddr
	"""
	def __init__(self, nextptr = None, conditions = [], actions = []):
		super(Trigger, self).__init__()
		
		conditions = FlattenList(conditions)
		actions = FlattenList(actions)
		
		
		# basic assert
		assert len(conditions) <= 16
		assert len(actions) <= 64
		
		
		
		for cond in conditions:
			assert type(cond) is Condition
			
		for act in actions:
			assert type(act) is Action
			
		# set
		if nextptr:
			assert IsValidExpr(nextptr), "nextptr is not an addressable object or expression."
		
		self._nextptr = nextptr
		self._conditions = conditions
		self._actions = actions
		self._nexttrgaddr = addressable.Addr()
		
		# set last trigger
		lasttrig = _last_trigger_stack.pop()
		if lasttrig:
			if lasttrig._nextptr is None:
				lasttrig._nextptr = addressable.Addr(self)
				lasttrig._nexttrgaddr << self
		_last_trigger_stack.append(self)
	
	
	
	# override
	"""
	Override of Addressable::SetAddress
	"""
	def SetAddress(self, address):
		super(Trigger, self).SetAddress(address)
		for i, cond in enumerate(self._conditions):
			cond.SetAddress(address + 8 + 20 * i)
			
		for i, act in enumerate(self._actions):
			act.SetAddress(address + 8 + 320 + 32 * i)
	
	# some helper func
	def GetNextPtrAddr(self):
		return addressable.Addr(self) + 4
	
	def GetConditionAddr(self, index):
		assert 0 <= index < 16
		return addressable.Addr(self) + 8 + 20 * index
	
	def GetActionAddr(self, index):
		assert 0 <= index < 64
		return addressable.Addr(self) + 8 + 320 + 32 * index
	
	
	# function needed for payloadmanager
	def GetDataSize(self):
		return 2408
	
	def IsIndependent(self):
		return True

	def GetDependencyList(self):
		deplist = depgraph.GetDependencyList(self._nextptr)
		for cond in self._conditions:
			deplist.extend(cond.GetDependencyList())

		for act in self._actions:
			deplist.extend(act.GetDependencyList())

		return deplist

	
	def WritePayloadChunk(self, buf):
		buf.EmitDword(0)
		if self._nextptr is None:
			buf.EmitDword(0xFFFFFFFF) # by default behavior. This will lead to crash
		else:
			buf.EmitDword(self._nextptr)

		for cond in self._conditions:
			cond.WritePayloadChunk(buf)

		buf.EmitBytes(bytes(20 * (16 - len(self._conditions))))

		for act in self._actions:
			act.WritePayloadChunk(buf)

		buf.EmitBytes(bytes(32 * (64 - len(self._actions))))
		
		# 04 00 00 00 means 'preserve trigger'.
		buf.EmitBytes(b'\x04\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0')





"""
Condition class. Immutable. Stock conditions are defined at stocktrg.
"""
class Condition(addressable.Addressable):
	def __init__(self, locid, player, amount, unitid, comparison, condtype, restype, flags):
		assert IsValidExpr(locid)
		assert IsValidExpr(player)
		assert IsValidExpr(amount)
		assert IsValidExpr(unitid)
		assert IsValidExpr(comparison)
		assert IsValidExpr(condtype)
		assert IsValidExpr(restype)
		assert IsValidExpr(flags)
		
		super(Condition, self).__init__()
		self._locid = locid
		self._player = player
		self._amount = amount
		self._unitid = unitid
		self._comparison = comparison
		self._condtype = condtype
		self._restype = restype
		self._flags = flags
		
	def Disable(self):
		self._flags |= 2

	# function needed for payloadmanager
	def GetDataSize(self):
		return 20
	
	def IsIndependent(self):
		return False
	
	def GetDependencyList(self):
		return [
			self._locid,
			self._player,
			self._amount,
			self._unitid,
			self._comparison,
			self._condtype,
			self._restype,
			self._flags,
		]

	def WritePayloadChunk(self, buf):
		buf.EmitDword (self._locid)
		buf.EmitDword (self._player)
		buf.EmitDword (self._amount)
		buf.EmitWord  (self._unitid)
		buf.EmitByte  (self._comparison)
		buf.EmitByte  (self._condtype)
		buf.EmitByte  (self._restype)
		buf.EmitByte  (self._flags)
		buf.EmitBytes (b'\0\0')
		
		
"""
Action class. Immutable. Stock actions are defined at stocktrg.
"""
class Action(addressable.Addressable):
	def __init__(self, locid1, strid, wavid, time, player1, player2, unitid, acttype, amount, flags):
		super(Action, self).__init__()
		
		assert IsValidExpr(locid1)
		assert IsValidExpr(strid)
		assert IsValidExpr(wavid)
		assert IsValidExpr(time)
		assert IsValidExpr(player1)
		assert IsValidExpr(player2)
		assert IsValidExpr(unitid)
		assert IsValidExpr(acttype)
		assert IsValidExpr(amount)
		assert IsValidExpr(flags)
		
		self._locid1 = locid1
		self._strid = strid
		self._wavid = wavid
		self._time = time
		self._player1 = player1
		self._player2 = player2
		self._unitid = unitid
		self._acttype = acttype
		self._amount = amount
		self._flags = flags
		
	def Disable(self):
		self._flags |= 2
		
	# function needed for payloadmanager
	def GetDataSize(self):
		return 32
	
	def IsIndependent(self):
		return False
	
	def GetDependencyList(self):
		return [
			self._locid1,
			self._strid,
			self._wavid,
			self._time,
			self._player1,
			self._player2,
			self._unitid,
			self._acttype,
			self._amount,
			self._flags,
		]


	def WritePayloadChunk(self, buf):
		buf.EmitDword (self._locid1)
		buf.EmitDword (self._strid)
		buf.EmitDword (self._wavid)
		buf.EmitDword (self._time)
		buf.EmitDword (self._player1)
		buf.EmitDword (self._player2)
		buf.EmitWord  (self._unitid)
		buf.EmitByte  (self._acttype)
		buf.EmitByte  (self._amount)
		buf.EmitByte  (self._flags)
		buf.EmitBytes (b'\0\0\0')
		

	
def Disabled(item):
	item.Disable()
	return item