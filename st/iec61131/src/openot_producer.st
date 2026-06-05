TYPE
    OPENOT_SOURCE_ENTRY : STRUCT
        Used : BOOL;
        SourceId : UDINT;
        NextSeq : ULINT;
    END_STRUCT;

    OPENOT_SOURCE_USED_TABLE : ARRAY[0..15] OF BOOL;
    OPENOT_SOURCE_ID_TABLE : ARRAY[0..15] OF UDINT;
    OPENOT_SOURCE_NEXT_SEQ_TABLE : ARRAY[0..15] OF ULINT;
END_TYPE

FUNCTION_BLOCK OPENOT_Producer
VAR_INPUT
    Execute : BOOL;
    SourceId : UDINT;
END_VAR
VAR_OUTPUT
    AssignedSeq : ULINT;
    Error : BOOL;
    SourceTableFull : BOOL;
    Done : BOOL;
    SourceCount : UINT;
    SourceUsed : OPENOT_SOURCE_USED_TABLE;
    SourceIds : OPENOT_SOURCE_ID_TABLE;
    SourceNextSeqs : OPENOT_SOURCE_NEXT_SEQ_TABLE;
    Ring : OPENOT_RING256_BUFFER;
    HeadAbs : ULINT;
    OldestAbs : ULINT;
    LostCount : ULINT;
    LastStartAbs : ULINT;
    LastEndAbs : ULINT;
    LossRanges : OPENOT_PRODUCER_LOSS_RANGES;
    LossRangeCount : UINT;
    LossRangeOverflow : BOOL;
    RingError : BOOL;
END_VAR
VAR
    Sources : ARRAY[0..15] OF OPENOT_SOURCE_ENTRY;
    RingProducer : OPENOT_Ring256Producer;
    Encoder : OPENOT_EncodeMinimalMessage;
    Index : UINT;
    ByteIndex : UINT;
    LossIndex : UINT;
    EntryIndex : UINT;
    Found : BOOL;
    FreeFound : BOOL;
END_VAR

Done := FALSE;
Error := FALSE;
SourceTableFull := FALSE;
AssignedSeq := ULINT#0;

IF Execute THEN
    IF SourceId = UDINT#0 THEN
        Error := TRUE;
    ELSE
        Found := FALSE;
        FreeFound := FALSE;
        EntryIndex := UINT#0;

        FOR Index := UINT#0 TO UINT#15 DO
            IF Sources[Index].Used AND (Sources[Index].SourceId = SourceId) THEN
                IF NOT Found THEN
                    EntryIndex := Index;
                    Found := TRUE;
                END_IF;
            END_IF;
        END_FOR;

        IF NOT Found THEN
            FOR Index := UINT#0 TO UINT#15 DO
                IF (NOT Sources[Index].Used) AND (NOT FreeFound) THEN
                    EntryIndex := Index;
                    FreeFound := TRUE;
                END_IF;
            END_FOR;

            IF FreeFound THEN
                Sources[EntryIndex].Used := TRUE;
                Sources[EntryIndex].SourceId := SourceId;
                Sources[EntryIndex].NextSeq := ULINT#0;
            ELSE
                SourceTableFull := TRUE;
                Error := TRUE;
            END_IF;
        END_IF;

        IF NOT Error THEN
            AssignedSeq := Sources[EntryIndex].NextSeq;
            Sources[EntryIndex].NextSeq := Sources[EntryIndex].NextSeq + ULINT#1;

            Encoder(SourceId := SourceId, Seq := AssignedSeq);
            RingProducer(
                Execute := TRUE,
                Record := Encoder.Buffer,
                Length := Encoder.Length,
                RunId := ULINT#1,
                SourceId := SourceId,
                Seq := AssignedSeq);

            IF RingProducer.Done AND (NOT RingProducer.Error) THEN
                Done := TRUE;
            ELSE
                Error := TRUE;
            END_IF;
        END_IF;
    END_IF;
END_IF;

SourceCount := UINT#0;
FOR Index := UINT#0 TO UINT#15 DO
    SourceUsed[Index] := Sources[Index].Used;
    SourceIds[Index] := Sources[Index].SourceId;
    SourceNextSeqs[Index] := Sources[Index].NextSeq;
    IF Sources[Index].Used THEN
        SourceCount := SourceCount + UINT#1;
    END_IF;
END_FOR;

FOR ByteIndex := UINT#0 TO UINT#255 DO
    Ring[ByteIndex] := RingProducer.Ring[ByteIndex];
END_FOR;

FOR LossIndex := UINT#0 TO UINT#15 DO
    LossRanges[LossIndex].RunId := RingProducer.LossRanges[LossIndex].RunId;
    LossRanges[LossIndex].SourceId := RingProducer.LossRanges[LossIndex].SourceId;
    LossRanges[LossIndex].FirstSeq := RingProducer.LossRanges[LossIndex].FirstSeq;
    LossRanges[LossIndex].LastSeq := RingProducer.LossRanges[LossIndex].LastSeq;
END_FOR;

HeadAbs := RingProducer.HeadAbs;
OldestAbs := RingProducer.OldestAbs;
LostCount := RingProducer.LostCount;
LastStartAbs := RingProducer.LastStartAbs;
LastEndAbs := RingProducer.LastEndAbs;
LossRangeCount := RingProducer.LossRangeCount;
LossRangeOverflow := RingProducer.LossRangeOverflow;
RingError := RingProducer.Error;
END_FUNCTION_BLOCK
