FUNCTION_BLOCK OPENOT_EncodeValueChangedReal
VAR_INPUT
    SourceId : UDINT;
    RunId : ULINT;
    Seq : ULINT;
    SourceTime : ULINT;
    ValueId : UDINT;
    NewValue : REAL;
    HasPrevious : BOOL;
    PreviousValue : REAL;
    HasQuality : BOOL;
    Quality : UINT;
END_VAR
VAR_OUTPUT
    Buffer : OPENOT_BYTE_BUFFER;
    Length : UINT;
END_VAR
VAR
    Pos : UINT;
    TotalLength : UINT;
    Crc : DWORD;
    ValueBits : DWORD;
END_VAR

TotalLength := UINT#60;
IF HasPrevious THEN
    TotalLength := TotalLength + UINT#8;
END_IF;
IF HasQuality THEN
    TotalLength := TotalLength + UINT#8;
END_IF;

Pos := OPENOT_WriteHeader(
    TotalLength := TotalLength,
    Flags := UINT#16#0008,
    SourceTime := SourceTime,
    RunId := RunId,
    Seq := Seq,
    SourceId := SourceId,
    EventTypeId := UDINT#16#0002,
    Buffer := Buffer);

Pos := OPENOT_WriteSlotHeader(Offset := Pos, Key := UINT#16#000D, TypeTag := BYTE#16#05, PayloadLength := BYTE#4, Buffer := Buffer);
Pos := OPENOT_WriteU32LE(Offset := Pos, Value := ValueId, Buffer := Buffer);

IF HasPrevious THEN
    ValueBits := REAL_TO_DWORD(PreviousValue);
    Pos := OPENOT_WriteSlotHeader(Offset := Pos, Key := UINT#16#000F, TypeTag := BYTE#16#09, PayloadLength := BYTE#4, Buffer := Buffer);
    Pos := OPENOT_WriteDWordLE(Offset := Pos, Value := ValueBits, Buffer := Buffer);
END_IF;

ValueBits := REAL_TO_DWORD(NewValue);
Pos := OPENOT_WriteSlotHeader(Offset := Pos, Key := UINT#16#0010, TypeTag := BYTE#16#09, PayloadLength := BYTE#4, Buffer := Buffer);
Pos := OPENOT_WriteDWordLE(Offset := Pos, Value := ValueBits, Buffer := Buffer);

IF HasQuality THEN
    Pos := OPENOT_WriteSlotHeader(Offset := Pos, Key := UINT#16#0011, TypeTag := BYTE#16#03, PayloadLength := BYTE#2, Buffer := Buffer);
    Pos := OPENOT_WriteU16LE(Offset := Pos, Value := Quality, Buffer := Buffer);
    Pos := OPENOT_WriteByte(Offset := Pos, Value := BYTE#16#00, Buffer := Buffer);
    Pos := OPENOT_WriteByte(Offset := Pos, Value := BYTE#16#00, Buffer := Buffer);
END_IF;

Crc := OPENOT_Crc32c(Data := Buffer, Length := Pos);
Pos := OPENOT_WriteDWordLE(Offset := Pos, Value := Crc, Buffer := Buffer);
Length := Pos;
END_FUNCTION_BLOCK

FUNCTION_BLOCK OPENOT_EncodeValueChangedInt
VAR_INPUT
    SourceId : UDINT;
    RunId : ULINT;
    Seq : ULINT;
    SourceTime : ULINT;
    ValueId : UDINT;
    NewValue : DINT;
    HasPrevious : BOOL;
    PreviousValue : DINT;
    HasQuality : BOOL;
    Quality : UINT;
END_VAR
VAR_OUTPUT
    Buffer : OPENOT_BYTE_BUFFER;
    Length : UINT;
END_VAR
VAR
    Pos : UINT;
    TotalLength : UINT;
    Crc : DWORD;
    ValueBits : DWORD;
END_VAR

TotalLength := UINT#60;
IF HasPrevious THEN
    TotalLength := TotalLength + UINT#8;
END_IF;
IF HasQuality THEN
    TotalLength := TotalLength + UINT#8;
END_IF;

Pos := OPENOT_WriteHeader(
    TotalLength := TotalLength,
    Flags := UINT#16#0008,
    SourceTime := SourceTime,
    RunId := RunId,
    Seq := Seq,
    SourceId := SourceId,
    EventTypeId := UDINT#16#0002,
    Buffer := Buffer);

Pos := OPENOT_WriteSlotHeader(Offset := Pos, Key := UINT#16#000D, TypeTag := BYTE#16#05, PayloadLength := BYTE#4, Buffer := Buffer);
Pos := OPENOT_WriteU32LE(Offset := Pos, Value := ValueId, Buffer := Buffer);

IF HasPrevious THEN
    ValueBits := DINT_TO_DWORD(PreviousValue);
    Pos := OPENOT_WriteSlotHeader(Offset := Pos, Key := UINT#16#000F, TypeTag := BYTE#16#06, PayloadLength := BYTE#4, Buffer := Buffer);
    Pos := OPENOT_WriteDWordLE(Offset := Pos, Value := ValueBits, Buffer := Buffer);
END_IF;

ValueBits := DINT_TO_DWORD(NewValue);
Pos := OPENOT_WriteSlotHeader(Offset := Pos, Key := UINT#16#0010, TypeTag := BYTE#16#06, PayloadLength := BYTE#4, Buffer := Buffer);
Pos := OPENOT_WriteDWordLE(Offset := Pos, Value := ValueBits, Buffer := Buffer);

IF HasQuality THEN
    Pos := OPENOT_WriteSlotHeader(Offset := Pos, Key := UINT#16#0011, TypeTag := BYTE#16#03, PayloadLength := BYTE#2, Buffer := Buffer);
    Pos := OPENOT_WriteU16LE(Offset := Pos, Value := Quality, Buffer := Buffer);
    Pos := OPENOT_WriteByte(Offset := Pos, Value := BYTE#16#00, Buffer := Buffer);
    Pos := OPENOT_WriteByte(Offset := Pos, Value := BYTE#16#00, Buffer := Buffer);
END_IF;

Crc := OPENOT_Crc32c(Data := Buffer, Length := Pos);
Pos := OPENOT_WriteDWordLE(Offset := Pos, Value := Crc, Buffer := Buffer);
Length := Pos;
END_FUNCTION_BLOCK

FUNCTION_BLOCK OPENOT_EncodeStateTransition
VAR_INPUT
    SourceId : UDINT;
    RunId : ULINT;
    Seq : ULINT;
    SourceTime : ULINT;
    StateMachineId : UDINT;
    Category : UINT;
    PreviousState : UINT;
    NewState : UINT;
END_VAR
VAR_OUTPUT
    Buffer : OPENOT_BYTE_BUFFER;
    Length : UINT;
END_VAR
VAR
    Pos : UINT;
    Crc : DWORD;
END_VAR

Pos := OPENOT_WriteHeader(
    TotalLength := UINT#76,
    Flags := UINT#16#0008,
    SourceTime := SourceTime,
    RunId := RunId,
    Seq := Seq,
    SourceId := SourceId,
    EventTypeId := UDINT#16#0001,
    Buffer := Buffer);

Pos := OPENOT_WriteSlotHeader(Offset := Pos, Key := UINT#16#0001, TypeTag := BYTE#16#05, PayloadLength := BYTE#4, Buffer := Buffer);
Pos := OPENOT_WriteU32LE(Offset := Pos, Value := StateMachineId, Buffer := Buffer);
Pos := OPENOT_WriteSlotHeader(Offset := Pos, Key := UINT#16#0002, TypeTag := BYTE#16#03, PayloadLength := BYTE#2, Buffer := Buffer);
Pos := OPENOT_WriteU16LE(Offset := Pos, Value := Category, Buffer := Buffer);
Pos := OPENOT_WriteByte(Offset := Pos, Value := BYTE#16#00, Buffer := Buffer);
Pos := OPENOT_WriteByte(Offset := Pos, Value := BYTE#16#00, Buffer := Buffer);
Pos := OPENOT_WriteSlotHeader(Offset := Pos, Key := UINT#16#0003, TypeTag := BYTE#16#03, PayloadLength := BYTE#2, Buffer := Buffer);
Pos := OPENOT_WriteU16LE(Offset := Pos, Value := PreviousState, Buffer := Buffer);
Pos := OPENOT_WriteByte(Offset := Pos, Value := BYTE#16#00, Buffer := Buffer);
Pos := OPENOT_WriteByte(Offset := Pos, Value := BYTE#16#00, Buffer := Buffer);
Pos := OPENOT_WriteSlotHeader(Offset := Pos, Key := UINT#16#0004, TypeTag := BYTE#16#03, PayloadLength := BYTE#2, Buffer := Buffer);
Pos := OPENOT_WriteU16LE(Offset := Pos, Value := NewState, Buffer := Buffer);
Pos := OPENOT_WriteByte(Offset := Pos, Value := BYTE#16#00, Buffer := Buffer);
Pos := OPENOT_WriteByte(Offset := Pos, Value := BYTE#16#00, Buffer := Buffer);
Crc := OPENOT_Crc32c(Data := Buffer, Length := Pos);
Pos := OPENOT_WriteDWordLE(Offset := Pos, Value := Crc, Buffer := Buffer);
Length := Pos;
END_FUNCTION_BLOCK

FUNCTION_BLOCK OPENOT_EncodeCondition
VAR_INPUT
    SourceId : UDINT;
    RunId : ULINT;
    Seq : ULINT;
    SourceTime : ULINT;
    EventTypeId : UDINT;
    ConditionId : UDINT;
    ConditionClass : UINT;
    Severity : UINT;
END_VAR
VAR_OUTPUT
    Buffer : OPENOT_BYTE_BUFFER;
    Length : UINT;
END_VAR
VAR
    Pos : UINT;
    Crc : DWORD;
END_VAR

Pos := OPENOT_WriteHeader(
    TotalLength := UINT#68,
    Flags := UINT#16#0008,
    SourceTime := SourceTime,
    RunId := RunId,
    Seq := Seq,
    SourceId := SourceId,
    EventTypeId := EventTypeId,
    Buffer := Buffer);

Pos := OPENOT_WriteSlotHeader(Offset := Pos, Key := UINT#16#0005, TypeTag := BYTE#16#05, PayloadLength := BYTE#4, Buffer := Buffer);
Pos := OPENOT_WriteU32LE(Offset := Pos, Value := ConditionId, Buffer := Buffer);
Pos := OPENOT_WriteSlotHeader(Offset := Pos, Key := UINT#16#0006, TypeTag := BYTE#16#03, PayloadLength := BYTE#2, Buffer := Buffer);
Pos := OPENOT_WriteU16LE(Offset := Pos, Value := ConditionClass, Buffer := Buffer);
Pos := OPENOT_WriteByte(Offset := Pos, Value := BYTE#16#00, Buffer := Buffer);
Pos := OPENOT_WriteByte(Offset := Pos, Value := BYTE#16#00, Buffer := Buffer);
Pos := OPENOT_WriteSlotHeader(Offset := Pos, Key := UINT#16#0008, TypeTag := BYTE#16#03, PayloadLength := BYTE#2, Buffer := Buffer);
Pos := OPENOT_WriteU16LE(Offset := Pos, Value := Severity, Buffer := Buffer);
Pos := OPENOT_WriteByte(Offset := Pos, Value := BYTE#16#00, Buffer := Buffer);
Pos := OPENOT_WriteByte(Offset := Pos, Value := BYTE#16#00, Buffer := Buffer);
Crc := OPENOT_Crc32c(Data := Buffer, Length := Pos);
Pos := OPENOT_WriteDWordLE(Offset := Pos, Value := Crc, Buffer := Buffer);
Length := Pos;
END_FUNCTION_BLOCK
