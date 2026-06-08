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

FUNCTION OPENOT_ValueSlotEncodedLength : UINT
VAR_INPUT
    PayloadLength : UINT;
END_VAR
VAR
    Padding : UINT;
END_VAR

Padding := UINT#0;
IF (PayloadLength MOD UINT#4) <> UINT#0 THEN
    Padding := UINT#4 - (PayloadLength MOD UINT#4);
END_IF;

OPENOT_ValueSlotEncodedLength := UINT#4 + PayloadLength + Padding;
END_FUNCTION

FUNCTION OPENOT_WriteValueBitsLE : UINT
VAR_INPUT
    Offset : UINT;
    Value : ULINT;
    PayloadLength : UINT;
END_VAR
VAR_IN_OUT
    Buffer : OPENOT_BYTE_BUFFER;
END_VAR
VAR
    Remaining : ULINT;
    Index : UINT;
    Pos : UINT;
END_VAR

Remaining := Value;
Pos := Offset;
FOR Index := UINT#0 TO PayloadLength - UINT#1 DO
    Pos := OPENOT_WriteByte(Offset := Pos, Value := ULINT_TO_BYTE(Remaining MOD ULINT#256), Buffer := Buffer);
    Remaining := Remaining / ULINT#256;
END_FOR;
WHILE (Pos MOD UINT#4) <> UINT#0 DO
    Pos := OPENOT_WriteByte(Offset := Pos, Value := BYTE#16#00, Buffer := Buffer);
END_WHILE;
OPENOT_WriteValueBitsLE := Pos;
END_FUNCTION

FUNCTION_BLOCK OPENOT_EncodeValueChangedBits
VAR_INPUT
    SourceId : UDINT;
    RunId : ULINT;
    Seq : ULINT;
    SourceTime : ULINT;
    ValueId : UDINT;
    TypeTag : BYTE;
    PayloadLength : UINT;
    NewBits : ULINT;
    HasPrevious : BOOL;
    PreviousBits : ULINT;
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
END_VAR

TotalLength := UINT#40 + UINT#8 + OPENOT_ValueSlotEncodedLength(PayloadLength := PayloadLength) + UINT#4;
IF HasPrevious THEN
    TotalLength := TotalLength + OPENOT_ValueSlotEncodedLength(PayloadLength := PayloadLength);
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
    Pos := OPENOT_WriteSlotHeader(Offset := Pos, Key := UINT#16#000F, TypeTag := TypeTag, PayloadLength := UINT_TO_BYTE(PayloadLength), Buffer := Buffer);
    Pos := OPENOT_WriteValueBitsLE(Offset := Pos, Value := PreviousBits, PayloadLength := PayloadLength, Buffer := Buffer);
END_IF;

Pos := OPENOT_WriteSlotHeader(Offset := Pos, Key := UINT#16#0010, TypeTag := TypeTag, PayloadLength := UINT_TO_BYTE(PayloadLength), Buffer := Buffer);
Pos := OPENOT_WriteValueBitsLE(Offset := Pos, Value := NewBits, PayloadLength := PayloadLength, Buffer := Buffer);

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

FUNCTION OPENOT_WriteStringPayload : UINT
VAR_INPUT
    Offset : UINT;
    Value : STRING[96];
    PayloadLength : UINT;
END_VAR
VAR_IN_OUT
    Buffer : OPENOT_BYTE_BUFFER;
END_VAR
VAR
    Index : UINT;
    Pos : UINT;
    Slice : STRING[1];
END_VAR

Pos := Offset;
IF PayloadLength > UINT#0 THEN
    FOR Index := UINT#0 TO PayloadLength - UINT#1 DO
        Slice := MID(IN := Value, L := INT#1, P := UINT_TO_INT(Index + UINT#1));
        Pos := OPENOT_WriteByte(Offset := Pos, Value := CHAR_TO_BYTE(STRING_TO_CHAR(Slice)), Buffer := Buffer);
    END_FOR;
END_IF;
WHILE (Pos MOD UINT#4) <> UINT#0 DO
    Pos := OPENOT_WriteByte(Offset := Pos, Value := BYTE#16#00, Buffer := Buffer);
END_WHILE;
OPENOT_WriteStringPayload := Pos;
END_FUNCTION

FUNCTION_BLOCK OPENOT_EncodeValueChangedString
VAR_INPUT
    SourceId : UDINT;
    RunId : ULINT;
    Seq : ULINT;
    SourceTime : ULINT;
    ValueId : UDINT;
    NewValue : STRING[96];
    HasPrevious : BOOL;
    PreviousValue : STRING[96];
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
    NewLength : UINT;
    PreviousLength : UINT;
    Crc : DWORD;
END_VAR

NewLength := INT_TO_UINT(LEN(NewValue));
PreviousLength := INT_TO_UINT(LEN(PreviousValue));
IF NewLength > UINT#96 THEN
    NewLength := UINT#96;
END_IF;
IF PreviousLength > UINT#96 THEN
    PreviousLength := UINT#96;
END_IF;

TotalLength := UINT#40 + UINT#8 + OPENOT_ValueSlotEncodedLength(PayloadLength := NewLength) + UINT#4;
IF HasPrevious THEN
    TotalLength := TotalLength + OPENOT_ValueSlotEncodedLength(PayloadLength := PreviousLength);
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
    Pos := OPENOT_WriteSlotHeader(Offset := Pos, Key := UINT#16#000F, TypeTag := BYTE#16#0C, PayloadLength := UINT_TO_BYTE(PreviousLength), Buffer := Buffer);
    Pos := OPENOT_WriteStringPayload(Offset := Pos, Value := PreviousValue, PayloadLength := PreviousLength, Buffer := Buffer);
END_IF;

Pos := OPENOT_WriteSlotHeader(Offset := Pos, Key := UINT#16#0010, TypeTag := BYTE#16#0C, PayloadLength := UINT_TO_BYTE(NewLength), Buffer := Buffer);
Pos := OPENOT_WriteStringPayload(Offset := Pos, Value := NewValue, PayloadLength := NewLength, Buffer := Buffer);

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
    HasCorrelationId : BOOL;
    CorrelationId : UDINT;
    HasCauseOperand : BOOL;
    CauseOperand : UDINT;
END_VAR
VAR_OUTPUT
    Buffer : OPENOT_BYTE_BUFFER;
    Length : UINT;
END_VAR
VAR
    Pos : UINT;
    Crc : DWORD;
    TotalLength : UINT;
END_VAR

TotalLength := UINT#68;
IF HasCorrelationId THEN
    TotalLength := TotalLength + UINT#8;
END_IF;
IF HasCauseOperand THEN
    TotalLength := TotalLength + UINT#8;
END_IF;

Pos := OPENOT_WriteHeader(
    TotalLength := TotalLength,
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
IF HasCorrelationId THEN
    Pos := OPENOT_WriteSlotHeader(Offset := Pos, Key := UINT#16#0007, TypeTag := BYTE#16#05, PayloadLength := BYTE#4, Buffer := Buffer);
    Pos := OPENOT_WriteU32LE(Offset := Pos, Value := CorrelationId, Buffer := Buffer);
END_IF;
IF HasCauseOperand THEN
    Pos := OPENOT_WriteSlotHeader(Offset := Pos, Key := UINT#16#0009, TypeTag := BYTE#16#05, PayloadLength := BYTE#4, Buffer := Buffer);
    Pos := OPENOT_WriteU32LE(Offset := Pos, Value := CauseOperand, Buffer := Buffer);
END_IF;
Crc := OPENOT_Crc32c(Data := Buffer, Length := Pos);
Pos := OPENOT_WriteDWordLE(Offset := Pos, Value := Crc, Buffer := Buffer);
Length := Pos;
END_FUNCTION_BLOCK
