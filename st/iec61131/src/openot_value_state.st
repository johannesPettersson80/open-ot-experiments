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

ValueBits := REAL_TO_DWORD(NewValue);
Pos := OPENOT_WriteSlotHeader(Offset := Pos, Key := UINT#16#0010, TypeTag := BYTE#16#09, PayloadLength := BYTE#4, Buffer := Buffer);
Pos := OPENOT_WriteDWordLE(Offset := Pos, Value := ValueBits, Buffer := Buffer);

IF HasPrevious THEN
    ValueBits := REAL_TO_DWORD(PreviousValue);
    Pos := OPENOT_WriteSlotHeader(Offset := Pos, Key := UINT#16#000F, TypeTag := BYTE#16#09, PayloadLength := BYTE#4, Buffer := Buffer);
    Pos := OPENOT_WriteDWordLE(Offset := Pos, Value := ValueBits, Buffer := Buffer);
END_IF;

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

FUNCTION_BLOCK OPENOT_EncodeConditionLifecycle
VAR_INPUT
    SourceId : UDINT;
    RunId : ULINT;
    Seq : ULINT;
    SourceTime : ULINT;
    EventTypeId : UDINT;
    ConditionId : UDINT;
    CorrelationId : UDINT;
    HasAckBy : BOOL;
    AckBy : STRING[96];
    HasShelveSecs : BOOL;
    ShelveSecs : UDINT;
    HasReason : BOOL;
    Reason : STRING[96];
    HasComment : BOOL;
    Comment : STRING[96];
    HasPreviousPriority : BOOL;
    PreviousPriority : UINT;
    HasNewPriority : BOOL;
    NewPriority : UINT;
    HasRefreshId : BOOL;
    RefreshId : UDINT;
END_VAR
VAR_OUTPUT
    Buffer : OPENOT_BYTE_BUFFER;
    Length : UINT;
END_VAR
VAR
    Pos : UINT;
    TotalLength : UINT;
    Crc : DWORD;
    AckByLength : UINT;
    ReasonLength : UINT;
    CommentLength : UINT;
    IsActivationScoped : BOOL;
END_VAR

AckByLength := INT_TO_UINT(LEN(AckBy));
ReasonLength := INT_TO_UINT(LEN(Reason));
CommentLength := INT_TO_UINT(LEN(Comment));
IF AckByLength > UINT#96 THEN
    AckByLength := UINT#96;
END_IF;
IF ReasonLength > UINT#96 THEN
    ReasonLength := UINT#96;
END_IF;
IF CommentLength > UINT#96 THEN
    CommentLength := UINT#96;
END_IF;

IsActivationScoped := (EventTypeId = UDINT#16#0202) OR (EventTypeId = UDINT#16#0203) OR (EventTypeId = UDINT#16#0204) OR (EventTypeId = UDINT#16#0205) OR (EventTypeId = UDINT#16#020A) OR (EventTypeId = UDINT#16#020B);
TotalLength := UINT#52;
IF IsActivationScoped THEN
    TotalLength := TotalLength + UINT#8;
END_IF;
IF HasComment THEN
    TotalLength := TotalLength + OPENOT_ValueSlotEncodedLength(PayloadLength := CommentLength);
END_IF;
IF HasAckBy THEN
    TotalLength := TotalLength + OPENOT_ValueSlotEncodedLength(PayloadLength := AckByLength);
END_IF;
IF HasShelveSecs THEN
    TotalLength := TotalLength + UINT#8;
END_IF;
IF HasReason THEN
    TotalLength := TotalLength + OPENOT_ValueSlotEncodedLength(PayloadLength := ReasonLength);
END_IF;
IF HasPreviousPriority THEN
    TotalLength := TotalLength + UINT#8;
END_IF;
IF HasNewPriority THEN
    TotalLength := TotalLength + UINT#8;
END_IF;
IF HasRefreshId THEN
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

IF IsActivationScoped THEN
    Pos := OPENOT_WriteSlotHeader(Offset := Pos, Key := UINT#16#0007, TypeTag := BYTE#16#05, PayloadLength := BYTE#4, Buffer := Buffer);
    Pos := OPENOT_WriteU32LE(Offset := Pos, Value := CorrelationId, Buffer := Buffer);
END_IF;

IF EventTypeId = UDINT#16#020A THEN
    IF HasComment THEN
        Pos := OPENOT_WriteSlotHeader(Offset := Pos, Key := UINT#16#0037, TypeTag := BYTE#16#0C, PayloadLength := UINT_TO_BYTE(CommentLength), Buffer := Buffer);
        Pos := OPENOT_WriteStringPayload(Offset := Pos, Value := Comment, PayloadLength := CommentLength, Buffer := Buffer);
    END_IF;

    IF HasAckBy THEN
        Pos := OPENOT_WriteSlotHeader(Offset := Pos, Key := UINT#16#001D, TypeTag := BYTE#16#0C, PayloadLength := UINT_TO_BYTE(AckByLength), Buffer := Buffer);
        Pos := OPENOT_WriteStringPayload(Offset := Pos, Value := AckBy, PayloadLength := AckByLength, Buffer := Buffer);
    END_IF;
ELSIF EventTypeId = UDINT#16#020C THEN
    IF HasPreviousPriority THEN
        Pos := OPENOT_WriteSlotHeader(Offset := Pos, Key := UINT#16#002E, TypeTag := BYTE#16#03, PayloadLength := BYTE#2, Buffer := Buffer);
        Pos := OPENOT_WriteU16LE(Offset := Pos, Value := PreviousPriority, Buffer := Buffer);
        Pos := OPENOT_WriteByte(Offset := Pos, Value := BYTE#16#00, Buffer := Buffer);
        Pos := OPENOT_WriteByte(Offset := Pos, Value := BYTE#16#00, Buffer := Buffer);
    END_IF;

    IF HasNewPriority THEN
        Pos := OPENOT_WriteSlotHeader(Offset := Pos, Key := UINT#16#002C, TypeTag := BYTE#16#03, PayloadLength := BYTE#2, Buffer := Buffer);
        Pos := OPENOT_WriteU16LE(Offset := Pos, Value := NewPriority, Buffer := Buffer);
        Pos := OPENOT_WriteByte(Offset := Pos, Value := BYTE#16#00, Buffer := Buffer);
        Pos := OPENOT_WriteByte(Offset := Pos, Value := BYTE#16#00, Buffer := Buffer);
    END_IF;

    IF HasAckBy THEN
        Pos := OPENOT_WriteSlotHeader(Offset := Pos, Key := UINT#16#001D, TypeTag := BYTE#16#0C, PayloadLength := UINT_TO_BYTE(AckByLength), Buffer := Buffer);
        Pos := OPENOT_WriteStringPayload(Offset := Pos, Value := AckBy, PayloadLength := AckByLength, Buffer := Buffer);
    END_IF;
ELSE
    IF HasAckBy THEN
        Pos := OPENOT_WriteSlotHeader(Offset := Pos, Key := UINT#16#001D, TypeTag := BYTE#16#0C, PayloadLength := UINT_TO_BYTE(AckByLength), Buffer := Buffer);
        Pos := OPENOT_WriteStringPayload(Offset := Pos, Value := AckBy, PayloadLength := AckByLength, Buffer := Buffer);
    END_IF;

    IF HasShelveSecs THEN
        Pos := OPENOT_WriteSlotHeader(Offset := Pos, Key := UINT#16#001E, TypeTag := BYTE#16#05, PayloadLength := BYTE#4, Buffer := Buffer);
        Pos := OPENOT_WriteU32LE(Offset := Pos, Value := ShelveSecs, Buffer := Buffer);
    END_IF;

    IF HasReason THEN
        Pos := OPENOT_WriteSlotHeader(Offset := Pos, Key := UINT#16#001F, TypeTag := BYTE#16#0C, PayloadLength := UINT_TO_BYTE(ReasonLength), Buffer := Buffer);
        Pos := OPENOT_WriteStringPayload(Offset := Pos, Value := Reason, PayloadLength := ReasonLength, Buffer := Buffer);
    END_IF;
END_IF;

IF HasRefreshId THEN
    Pos := OPENOT_WriteSlotHeader(Offset := Pos, Key := UINT#16#0034, TypeTag := BYTE#16#05, PayloadLength := BYTE#4, Buffer := Buffer);
    Pos := OPENOT_WriteU32LE(Offset := Pos, Value := RefreshId, Buffer := Buffer);
END_IF;

Crc := OPENOT_Crc32c(Data := Buffer, Length := Pos);
Pos := OPENOT_WriteDWordLE(Offset := Pos, Value := Crc, Buffer := Buffer);
Length := Pos;
END_FUNCTION_BLOCK

FUNCTION_BLOCK OPENOT_EncodeConformantConditionAcknowledged
VAR_OUTPUT
    Buffer : OPENOT_BYTE_BUFFER;
    Length : UINT;
END_VAR
VAR
    Encoder : OPENOT_EncodeConditionLifecycle;
END_VAR

Encoder(
    SourceId := UDINT#66,
    RunId := ULINT#1,
    Seq := ULINT#17,
    SourceTime := ULINT#1000000160,
    EventTypeId := UDINT#16#0202,
    ConditionId := UDINT#9001,
    CorrelationId := UDINT#77,
    HasAckBy := TRUE,
    AckBy := 'operator-a',
    HasShelveSecs := FALSE,
    ShelveSecs := UDINT#0,
    HasReason := FALSE,
    Reason := '',
    HasComment := FALSE,
    Comment := '',
    HasPreviousPriority := FALSE,
    PreviousPriority := UINT#0,
    HasNewPriority := FALSE,
    NewPriority := UINT#0,
    HasRefreshId := FALSE,
    RefreshId := UDINT#0);
Buffer := Encoder.Buffer;
Length := Encoder.Length;
END_FUNCTION_BLOCK

FUNCTION_BLOCK OPENOT_EncodeConformantConditionShelved
VAR_OUTPUT
    Buffer : OPENOT_BYTE_BUFFER;
    Length : UINT;
END_VAR
VAR
    Encoder : OPENOT_EncodeConditionLifecycle;
END_VAR

Encoder(
    SourceId := UDINT#66,
    RunId := ULINT#1,
    Seq := ULINT#18,
    SourceTime := ULINT#1000000170,
    EventTypeId := UDINT#16#0204,
    ConditionId := UDINT#9001,
    CorrelationId := UDINT#77,
    HasAckBy := TRUE,
    AckBy := 'operator-a',
    HasShelveSecs := TRUE,
    ShelveSecs := UDINT#300,
    HasReason := FALSE,
    Reason := '',
    HasComment := FALSE,
    Comment := '',
    HasPreviousPriority := FALSE,
    PreviousPriority := UINT#0,
    HasNewPriority := FALSE,
    NewPriority := UINT#0,
    HasRefreshId := FALSE,
    RefreshId := UDINT#0);
Buffer := Encoder.Buffer;
Length := Encoder.Length;
END_FUNCTION_BLOCK

FUNCTION_BLOCK OPENOT_EncodeConformantConditionSuppressed
VAR_OUTPUT
    Buffer : OPENOT_BYTE_BUFFER;
    Length : UINT;
END_VAR
VAR
    Encoder : OPENOT_EncodeConditionLifecycle;
END_VAR

Encoder(
    SourceId := UDINT#66,
    RunId := ULINT#1,
    Seq := ULINT#19,
    SourceTime := ULINT#1000000180,
    EventTypeId := UDINT#16#0206,
    ConditionId := UDINT#9001,
    CorrelationId := UDINT#0,
    HasAckBy := FALSE,
    AckBy := '',
    HasShelveSecs := FALSE,
    ShelveSecs := UDINT#0,
    HasReason := TRUE,
    Reason := 'maintenance',
    HasComment := FALSE,
    Comment := '',
    HasPreviousPriority := FALSE,
    PreviousPriority := UINT#0,
    HasNewPriority := FALSE,
    NewPriority := UINT#0,
    HasRefreshId := FALSE,
    RefreshId := UDINT#0);
Buffer := Encoder.Buffer;
Length := Encoder.Length;
END_FUNCTION_BLOCK

FUNCTION_BLOCK OPENOT_EncodeConformantConditionOutOfService
VAR_OUTPUT
    Buffer : OPENOT_BYTE_BUFFER;
    Length : UINT;
END_VAR
VAR
    Encoder : OPENOT_EncodeConditionLifecycle;
END_VAR

Encoder(
    SourceId := UDINT#66,
    RunId := ULINT#1,
    Seq := ULINT#20,
    SourceTime := ULINT#1000000190,
    EventTypeId := UDINT#16#0208,
    ConditionId := UDINT#9001,
    CorrelationId := UDINT#0,
    HasAckBy := TRUE,
    AckBy := 'operator-a',
    HasShelveSecs := FALSE,
    ShelveSecs := UDINT#0,
    HasReason := FALSE,
    Reason := '',
    HasComment := FALSE,
    Comment := '',
    HasPreviousPriority := FALSE,
    PreviousPriority := UINT#0,
    HasNewPriority := FALSE,
    NewPriority := UINT#0,
    HasRefreshId := FALSE,
    RefreshId := UDINT#0);
Buffer := Encoder.Buffer;
Length := Encoder.Length;
END_FUNCTION_BLOCK

FUNCTION_BLOCK OPENOT_EncodeConformantConditionConfirmed
VAR_OUTPUT
    Buffer : OPENOT_BYTE_BUFFER;
    Length : UINT;
END_VAR
VAR
    Encoder : OPENOT_EncodeConditionLifecycle;
END_VAR

Encoder(
    SourceId := UDINT#66,
    RunId := ULINT#1,
    Seq := ULINT#21,
    SourceTime := ULINT#1000000200,
    EventTypeId := UDINT#16#0203,
    ConditionId := UDINT#9001,
    CorrelationId := UDINT#77,
    HasAckBy := TRUE,
    AckBy := 'operator-a',
    HasShelveSecs := FALSE,
    ShelveSecs := UDINT#0,
    HasReason := FALSE,
    Reason := '',
    HasComment := FALSE,
    Comment := '',
    HasPreviousPriority := FALSE,
    PreviousPriority := UINT#0,
    HasNewPriority := FALSE,
    NewPriority := UINT#0,
    HasRefreshId := FALSE,
    RefreshId := UDINT#0);
Buffer := Encoder.Buffer;
Length := Encoder.Length;
END_FUNCTION_BLOCK

FUNCTION_BLOCK OPENOT_EncodeConformantConditionUnshelved
VAR_OUTPUT
    Buffer : OPENOT_BYTE_BUFFER;
    Length : UINT;
END_VAR
VAR
    Encoder : OPENOT_EncodeConditionLifecycle;
END_VAR

Encoder(
    SourceId := UDINT#66,
    RunId := ULINT#1,
    Seq := ULINT#22,
    SourceTime := ULINT#1000000210,
    EventTypeId := UDINT#16#0205,
    ConditionId := UDINT#9001,
    CorrelationId := UDINT#77,
    HasAckBy := FALSE,
    AckBy := '',
    HasShelveSecs := FALSE,
    ShelveSecs := UDINT#0,
    HasReason := FALSE,
    Reason := '',
    HasComment := FALSE,
    Comment := '',
    HasPreviousPriority := FALSE,
    PreviousPriority := UINT#0,
    HasNewPriority := FALSE,
    NewPriority := UINT#0,
    HasRefreshId := FALSE,
    RefreshId := UDINT#0);
Buffer := Encoder.Buffer;
Length := Encoder.Length;
END_FUNCTION_BLOCK

FUNCTION_BLOCK OPENOT_EncodeConformantConditionUnsuppressed
VAR_OUTPUT
    Buffer : OPENOT_BYTE_BUFFER;
    Length : UINT;
END_VAR
VAR
    Encoder : OPENOT_EncodeConditionLifecycle;
END_VAR

Encoder(
    SourceId := UDINT#66,
    RunId := ULINT#1,
    Seq := ULINT#23,
    SourceTime := ULINT#1000000220,
    EventTypeId := UDINT#16#0207,
    ConditionId := UDINT#9001,
    CorrelationId := UDINT#0,
    HasAckBy := FALSE,
    AckBy := '',
    HasShelveSecs := FALSE,
    ShelveSecs := UDINT#0,
    HasReason := FALSE,
    Reason := '',
    HasComment := FALSE,
    Comment := '',
    HasPreviousPriority := FALSE,
    PreviousPriority := UINT#0,
    HasNewPriority := FALSE,
    NewPriority := UINT#0,
    HasRefreshId := FALSE,
    RefreshId := UDINT#0);
Buffer := Encoder.Buffer;
Length := Encoder.Length;
END_FUNCTION_BLOCK

FUNCTION_BLOCK OPENOT_EncodeConformantConditionInService
VAR_OUTPUT
    Buffer : OPENOT_BYTE_BUFFER;
    Length : UINT;
END_VAR
VAR
    Encoder : OPENOT_EncodeConditionLifecycle;
END_VAR

Encoder(
    SourceId := UDINT#66,
    RunId := ULINT#1,
    Seq := ULINT#24,
    SourceTime := ULINT#1000000230,
    EventTypeId := UDINT#16#0209,
    ConditionId := UDINT#9001,
    CorrelationId := UDINT#0,
    HasAckBy := FALSE,
    AckBy := '',
    HasShelveSecs := FALSE,
    ShelveSecs := UDINT#0,
    HasReason := FALSE,
    Reason := '',
    HasComment := FALSE,
    Comment := '',
    HasPreviousPriority := FALSE,
    PreviousPriority := UINT#0,
    HasNewPriority := FALSE,
    NewPriority := UINT#0,
    HasRefreshId := FALSE,
    RefreshId := UDINT#0);
Buffer := Encoder.Buffer;
Length := Encoder.Length;
END_FUNCTION_BLOCK

FUNCTION_BLOCK OPENOT_EncodeConformantConditionReset
VAR_OUTPUT
    Buffer : OPENOT_BYTE_BUFFER;
    Length : UINT;
END_VAR
VAR
    Encoder : OPENOT_EncodeConditionLifecycle;
END_VAR

Encoder(
    SourceId := UDINT#66,
    RunId := ULINT#1,
    Seq := ULINT#25,
    SourceTime := ULINT#1000000240,
    EventTypeId := UDINT#16#020B,
    ConditionId := UDINT#9001,
    CorrelationId := UDINT#77,
    HasAckBy := TRUE,
    AckBy := 'operator-a',
    HasShelveSecs := FALSE,
    ShelveSecs := UDINT#0,
    HasReason := FALSE,
    Reason := '',
    HasComment := FALSE,
    Comment := '',
    HasPreviousPriority := FALSE,
    PreviousPriority := UINT#0,
    HasNewPriority := FALSE,
    NewPriority := UINT#0,
    HasRefreshId := FALSE,
    RefreshId := UDINT#0);
Buffer := Encoder.Buffer;
Length := Encoder.Length;
END_FUNCTION_BLOCK

FUNCTION_BLOCK OPENOT_EncodeConformantConditionCommented
VAR_OUTPUT
    Buffer : OPENOT_BYTE_BUFFER;
    Length : UINT;
END_VAR
VAR
    Encoder : OPENOT_EncodeConditionLifecycle;
END_VAR

Encoder(
    SourceId := UDINT#66,
    RunId := ULINT#1,
    Seq := ULINT#26,
    SourceTime := ULINT#1000000250,
    EventTypeId := UDINT#16#020A,
    ConditionId := UDINT#9001,
    CorrelationId := UDINT#77,
    HasAckBy := TRUE,
    AckBy := 'operator-a',
    HasShelveSecs := FALSE,
    ShelveSecs := UDINT#0,
    HasReason := FALSE,
    Reason := '',
    HasComment := TRUE,
    Comment := 'operator comment',
    HasPreviousPriority := FALSE,
    PreviousPriority := UINT#0,
    HasNewPriority := FALSE,
    NewPriority := UINT#0,
    HasRefreshId := FALSE,
    RefreshId := UDINT#0);
Buffer := Encoder.Buffer;
Length := Encoder.Length;
END_FUNCTION_BLOCK

FUNCTION_BLOCK OPENOT_EncodeConformantConditionPriorityChanged
VAR_OUTPUT
    Buffer : OPENOT_BYTE_BUFFER;
    Length : UINT;
END_VAR
VAR
    Encoder : OPENOT_EncodeConditionLifecycle;
END_VAR

Encoder(
    SourceId := UDINT#66,
    RunId := ULINT#1,
    Seq := ULINT#27,
    SourceTime := ULINT#1000000260,
    EventTypeId := UDINT#16#020C,
    ConditionId := UDINT#9001,
    CorrelationId := UDINT#0,
    HasAckBy := TRUE,
    AckBy := 'operator-a',
    HasShelveSecs := FALSE,
    ShelveSecs := UDINT#0,
    HasReason := FALSE,
    Reason := '',
    HasComment := FALSE,
    Comment := '',
    HasPreviousPriority := TRUE,
    PreviousPriority := UINT#600,
    HasNewPriority := TRUE,
    NewPriority := UINT#900,
    HasRefreshId := FALSE,
    RefreshId := UDINT#0);
Buffer := Encoder.Buffer;
Length := Encoder.Length;
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

ValueBits := DINT_TO_DWORD(NewValue);
Pos := OPENOT_WriteSlotHeader(Offset := Pos, Key := UINT#16#0010, TypeTag := BYTE#16#06, PayloadLength := BYTE#4, Buffer := Buffer);
Pos := OPENOT_WriteDWordLE(Offset := Pos, Value := ValueBits, Buffer := Buffer);

IF HasPrevious THEN
    ValueBits := DINT_TO_DWORD(PreviousValue);
    Pos := OPENOT_WriteSlotHeader(Offset := Pos, Key := UINT#16#000F, TypeTag := BYTE#16#06, PayloadLength := BYTE#4, Buffer := Buffer);
    Pos := OPENOT_WriteDWordLE(Offset := Pos, Value := ValueBits, Buffer := Buffer);
END_IF;

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

Pos := OPENOT_WriteSlotHeader(Offset := Pos, Key := UINT#16#0010, TypeTag := TypeTag, PayloadLength := UINT_TO_BYTE(PayloadLength), Buffer := Buffer);
Pos := OPENOT_WriteValueBitsLE(Offset := Pos, Value := NewBits, PayloadLength := PayloadLength, Buffer := Buffer);

IF HasPrevious THEN
    Pos := OPENOT_WriteSlotHeader(Offset := Pos, Key := UINT#16#000F, TypeTag := TypeTag, PayloadLength := UINT_TO_BYTE(PayloadLength), Buffer := Buffer);
    Pos := OPENOT_WriteValueBitsLE(Offset := Pos, Value := PreviousBits, PayloadLength := PayloadLength, Buffer := Buffer);
END_IF;

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

Pos := OPENOT_WriteSlotHeader(Offset := Pos, Key := UINT#16#0010, TypeTag := BYTE#16#0C, PayloadLength := UINT_TO_BYTE(NewLength), Buffer := Buffer);
Pos := OPENOT_WriteStringPayload(Offset := Pos, Value := NewValue, PayloadLength := NewLength, Buffer := Buffer);

IF HasPrevious THEN
    Pos := OPENOT_WriteSlotHeader(Offset := Pos, Key := UINT#16#000F, TypeTag := BYTE#16#0C, PayloadLength := UINT_TO_BYTE(PreviousLength), Buffer := Buffer);
    Pos := OPENOT_WriteStringPayload(Offset := Pos, Value := PreviousValue, PayloadLength := PreviousLength, Buffer := Buffer);
END_IF;

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

TotalLength := UINT#60;
IF HasCorrelationId THEN
    TotalLength := TotalLength + UINT#8;
END_IF;
IF EventTypeId = UDINT#16#0200 THEN
    TotalLength := TotalLength + UINT#8;
END_IF;
IF (EventTypeId = UDINT#16#0200) AND HasCauseOperand THEN
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
IF HasCorrelationId THEN
    Pos := OPENOT_WriteSlotHeader(Offset := Pos, Key := UINT#16#0007, TypeTag := BYTE#16#05, PayloadLength := BYTE#4, Buffer := Buffer);
    Pos := OPENOT_WriteU32LE(Offset := Pos, Value := CorrelationId, Buffer := Buffer);
END_IF;
IF EventTypeId = UDINT#16#0200 THEN
    Pos := OPENOT_WriteSlotHeader(Offset := Pos, Key := UINT#16#0008, TypeTag := BYTE#16#03, PayloadLength := BYTE#2, Buffer := Buffer);
    Pos := OPENOT_WriteU16LE(Offset := Pos, Value := Severity, Buffer := Buffer);
    Pos := OPENOT_WriteByte(Offset := Pos, Value := BYTE#16#00, Buffer := Buffer);
    Pos := OPENOT_WriteByte(Offset := Pos, Value := BYTE#16#00, Buffer := Buffer);
END_IF;
IF (EventTypeId = UDINT#16#0200) AND HasCauseOperand THEN
    Pos := OPENOT_WriteSlotHeader(Offset := Pos, Key := UINT#16#0009, TypeTag := BYTE#16#05, PayloadLength := BYTE#4, Buffer := Buffer);
    Pos := OPENOT_WriteU32LE(Offset := Pos, Value := CauseOperand, Buffer := Buffer);
END_IF;
Crc := OPENOT_Crc32c(Data := Buffer, Length := Pos);
Pos := OPENOT_WriteDWordLE(Offset := Pos, Value := Crc, Buffer := Buffer);
Length := Pos;
END_FUNCTION_BLOCK
