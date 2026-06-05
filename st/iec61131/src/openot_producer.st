TYPE
    OPENOT_SOURCE_ENTRY : STRUCT
        Used : BOOL;
        SourceId : UDINT;
        NextSeq : ULINT;
    END_STRUCT;

    OPENOT_SOURCE_USED_TABLE : ARRAY[0..15] OF BOOL;
    OPENOT_SOURCE_ID_TABLE : ARRAY[0..15] OF UDINT;
    OPENOT_SOURCE_NEXT_SEQ_TABLE : ARRAY[0..15] OF ULINT;

    OPENOT_EMISSION_ENTRY : STRUCT
        EventTypeId : UDINT;
        RunId : ULINT;
        SourceId : UDINT;
        Seq : ULINT;
    END_STRUCT;

    OPENOT_EMISSION_LOG : ARRAY[0..31] OF OPENOT_EMISSION_ENTRY;
END_TYPE

FUNCTION_BLOCK OPENOT_Producer
VAR_INPUT
    Execute : BOOL;
    Op : UINT;
    SourceId : UDINT;
    Checkpoint : BOOL;
    DefHashOld : OPENOT_HASH8;
    DefHashNew : OPENOT_HASH8;
    TransitionEpochId : ULINT;
END_VAR
VAR_OUTPUT
    AssignedSeq : ULINT;
    ProducedCount : ULINT;
    Error : BOOL;
    SourceTableFull : BOOL;
    Done : BOOL;
    CheckpointDone : BOOL;
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
    RunId : ULINT;
    SourceTime : ULINT;
    SystemSeq : ULINT;
    EpochId : ULINT;
    EpochFirstAbs : ULINT;
    DefinitionHash : OPENOT_HASH8;
    PrevDefinitionHash : OPENOT_HASH8;
    TransitionOpen : BOOL;
    DefinitionChangedEmitted : BOOL;
    Emissions : OPENOT_EMISSION_LOG;
    EmissionCount : UINT;
    PublishedRecordCount : ULINT;
END_VAR
VAR
    Sources : ARRAY[0..15] OF OPENOT_SOURCE_ENTRY;
    RingProducer : OPENOT_Ring256Producer;
    Encoder : OPENOT_EncodeMinimalMessage;
    HighWaterEncoder : OPENOT_EncodeSourceHighWater;
    StoppedEncoder : OPENOT_EncodeLoggerStopped;
    StartedEncoder : OPENOT_EncodeLoggerStarted;
    DefinitionEncoder : OPENOT_EncodeDefinitionChanged;
    Index : UINT;
    ByteIndex : UINT;
    LossIndex : UINT;
    EntryIndex : UINT;
    Found : BOOL;
    FreeFound : BOOL;
    Initialized : BOOL;
    PreviousExecute : BOOL;
    ExecuteEdge : BOOL;
    StateRunId : ULINT;
    StateSourceTime : ULINT;
    StateSystemSeq : ULINT;
    StateEpochId : ULINT;
    StateEpochFirstAbs : ULINT;
    StateDefinitionHash : OPENOT_HASH8;
    StatePrevDefinitionHash : OPENOT_HASH8;
    StatePublishedRecordCount : ULINT;
    PendingDefHashOld : OPENOT_HASH8;
    PendingDefHashNew : OPENOT_HASH8;
    PendingEpochId : ULINT;
    PendingOpen : BOOL;
    PendingDefChangedEmitted : BOOL;
    EmitSeq : ULINT;
    EmitSourceTime : ULINT;
    SourceLoopActive : BOOL;
    HasLastSourceId : BOOL;
    LastSourceId : UDINT;
    NextSourceId : UDINT;
    SelectedIndex : UINT;
    CandidateFound : BOOL;
END_VAR

IF NOT Initialized THEN
    StateRunId := ULINT#1;
    StateSourceTime := ULINT#1780000000000000000;
    StateSystemSeq := ULINT#0;
    StateEpochId := ULINT#1;
    StateEpochFirstAbs := ULINT#0;
    StatePublishedRecordCount := ULINT#0;
    Initialized := TRUE;
END_IF;

Done := FALSE;
Error := FALSE;
SourceTableFull := FALSE;
AssignedSeq := ULINT#0;
ProducedCount := ULINT#0;
CheckpointDone := FALSE;
ExecuteEdge := Execute AND (NOT PreviousExecute);

IF ExecuteEdge THEN
    IF Checkpoint THEN
        IF PendingOpen THEN
            Error := TRUE;
        ELSIF SourceId = UDINT#0 THEN
            Error := TRUE;
        ELSE
            Found := FALSE;
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
                Error := TRUE;
            ELSE
                ProducedCount := Sources[EntryIndex].NextSeq;
                EmitSourceTime := StateSourceTime;
                StateSourceTime := StateSourceTime + ULINT#1;
                HighWaterEncoder(
                    SourceId := SourceId,
                    RunId := StateRunId,
                    ProducedCount := ProducedCount,
                    SourceTime := EmitSourceTime);
                RingProducer(
                    Execute := TRUE,
                    Record := HighWaterEncoder.Buffer,
                    Length := HighWaterEncoder.Length,
                    RunId := StateRunId,
                    SourceId := SourceId,
                    Seq := ProducedCount);

                IF RingProducer.Done AND (NOT RingProducer.Error) THEN
                    Sources[EntryIndex].NextSeq := ProducedCount + ULINT#1;
                    StatePublishedRecordCount := StatePublishedRecordCount + ULINT#1;
                    IF EmissionCount < UINT#32 THEN
                        Emissions[EmissionCount].EventTypeId := UDINT#16#0108;
                        Emissions[EmissionCount].RunId := StateRunId;
                        Emissions[EmissionCount].SourceId := SourceId;
                        Emissions[EmissionCount].Seq := ProducedCount;
                        EmissionCount := EmissionCount + UINT#1;
                    END_IF;
                    Done := TRUE;
                    CheckpointDone := TRUE;
                ELSE
                    Error := TRUE;
                END_IF;
            END_IF;
        END_IF;
    ELSIF Op = UINT#0 THEN
        IF PendingOpen THEN
            Error := TRUE;
        ELSIF SourceId = UDINT#0 THEN
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
                EmitSourceTime := StateSourceTime;
                StateSourceTime := StateSourceTime + ULINT#1;

                Encoder(
                    SourceId := SourceId,
                    RunId := StateRunId,
                    Seq := AssignedSeq,
                    SourceTime := EmitSourceTime);
                RingProducer(
                    Execute := TRUE,
                    Record := Encoder.Buffer,
                    Length := Encoder.Length,
                    RunId := StateRunId,
                    SourceId := SourceId,
                    Seq := AssignedSeq);

                IF RingProducer.Done AND (NOT RingProducer.Error) THEN
                    StatePublishedRecordCount := StatePublishedRecordCount + ULINT#1;
                    IF EmissionCount < UINT#32 THEN
                        Emissions[EmissionCount].EventTypeId := UDINT#16#0003;
                        Emissions[EmissionCount].RunId := StateRunId;
                        Emissions[EmissionCount].SourceId := SourceId;
                        Emissions[EmissionCount].Seq := AssignedSeq;
                        EmissionCount := EmissionCount + UINT#1;
                    END_IF;
                    Done := TRUE;
                ELSE
                    Error := TRUE;
                END_IF;
            END_IF;
        END_IF;
    ELSIF Op = UINT#1 THEN
        IF PendingOpen THEN
            Error := TRUE;
        ELSE
            EmitSeq := StateSystemSeq;
            StateSystemSeq := StateSystemSeq + ULINT#1;
            EmitSourceTime := StateSourceTime;
            StateSourceTime := StateSourceTime + ULINT#1;
            StoppedEncoder(RunId := StateRunId, Seq := EmitSeq, SourceTime := EmitSourceTime);
            RingProducer(
                Execute := TRUE,
                Record := StoppedEncoder.Buffer,
                Length := StoppedEncoder.Length,
                RunId := StateRunId,
                SourceId := UDINT#0,
                Seq := EmitSeq);

            IF RingProducer.Done AND (NOT RingProducer.Error) THEN
                StatePublishedRecordCount := StatePublishedRecordCount + ULINT#1;
                IF EmissionCount < UINT#32 THEN
                    Emissions[EmissionCount].EventTypeId := UDINT#16#0102;
                    Emissions[EmissionCount].RunId := StateRunId;
                    Emissions[EmissionCount].SourceId := UDINT#0;
                    Emissions[EmissionCount].Seq := EmitSeq;
                    EmissionCount := EmissionCount + UINT#1;
                END_IF;
            ELSE
                Error := TRUE;
            END_IF;

            SourceLoopActive := NOT Error;
            HasLastSourceId := FALSE;
            LastSourceId := UDINT#0;

            WHILE SourceLoopActive DO
                CandidateFound := FALSE;
                SelectedIndex := UINT#0;
                NextSourceId := UDINT#0;

                FOR Index := UINT#0 TO UINT#15 DO
                    IF Sources[Index].Used AND ((NOT HasLastSourceId) OR (Sources[Index].SourceId > LastSourceId)) THEN
                        IF (NOT CandidateFound) OR (Sources[Index].SourceId < NextSourceId) THEN
                            CandidateFound := TRUE;
                            SelectedIndex := Index;
                            NextSourceId := Sources[Index].SourceId;
                        END_IF;
                    END_IF;
                END_FOR;

                IF CandidateFound THEN
                    ProducedCount := Sources[SelectedIndex].NextSeq;
                    EmitSourceTime := StateSourceTime;
                    StateSourceTime := StateSourceTime + ULINT#1;
                    HighWaterEncoder(
                        SourceId := Sources[SelectedIndex].SourceId,
                        RunId := StateRunId,
                        ProducedCount := ProducedCount,
                        SourceTime := EmitSourceTime);
                    RingProducer(
                        Execute := TRUE,
                        Record := HighWaterEncoder.Buffer,
                        Length := HighWaterEncoder.Length,
                        RunId := StateRunId,
                        SourceId := Sources[SelectedIndex].SourceId,
                        Seq := ProducedCount);

                    IF RingProducer.Done AND (NOT RingProducer.Error) THEN
                        Sources[SelectedIndex].NextSeq := ProducedCount + ULINT#1;
                        StatePublishedRecordCount := StatePublishedRecordCount + ULINT#1;
                        IF EmissionCount < UINT#32 THEN
                            Emissions[EmissionCount].EventTypeId := UDINT#16#0108;
                            Emissions[EmissionCount].RunId := StateRunId;
                            Emissions[EmissionCount].SourceId := Sources[SelectedIndex].SourceId;
                            Emissions[EmissionCount].Seq := ProducedCount;
                            EmissionCount := EmissionCount + UINT#1;
                        END_IF;
                        LastSourceId := NextSourceId;
                        HasLastSourceId := TRUE;
                    ELSE
                        Error := TRUE;
                        SourceLoopActive := FALSE;
                    END_IF;
                ELSE
                    SourceLoopActive := FALSE;
                END_IF;
            END_WHILE;

            IF NOT Error THEN
                FOR Index := UINT#0 TO UINT#7 DO
                    PendingDefHashOld[Index] := DefHashOld[Index];
                    PendingDefHashNew[Index] := DefHashNew[Index];
                END_FOR;
                PendingEpochId := TransitionEpochId;
                PendingDefChangedEmitted := FALSE;
                PendingOpen := TRUE;
                Done := TRUE;
            END_IF;
        END_IF;
    ELSIF Op = UINT#2 THEN
        IF NOT PendingOpen THEN
            Error := TRUE;
        ELSIF PendingDefChangedEmitted THEN
            Error := TRUE;
        ELSE
            EmitSeq := StateSystemSeq;
            StateSystemSeq := StateSystemSeq + ULINT#1;
            EmitSourceTime := StateSourceTime;
            StateSourceTime := StateSourceTime + ULINT#1;
            DefinitionEncoder(
                RunId := StateRunId,
                Seq := EmitSeq,
                DefHashOld := PendingDefHashOld,
                DefHashNew := PendingDefHashNew,
                EpochId := PendingEpochId,
                SourceTime := EmitSourceTime);
            RingProducer(
                Execute := TRUE,
                Record := DefinitionEncoder.Buffer,
                Length := DefinitionEncoder.Length,
                RunId := StateRunId,
                SourceId := UDINT#0,
                Seq := EmitSeq);

            IF RingProducer.Done AND (NOT RingProducer.Error) THEN
                StatePublishedRecordCount := StatePublishedRecordCount + ULINT#1;
                IF EmissionCount < UINT#32 THEN
                    Emissions[EmissionCount].EventTypeId := UDINT#16#0106;
                    Emissions[EmissionCount].RunId := StateRunId;
                    Emissions[EmissionCount].SourceId := UDINT#0;
                    Emissions[EmissionCount].Seq := EmitSeq;
                    EmissionCount := EmissionCount + UINT#1;
                END_IF;
                PendingDefChangedEmitted := TRUE;
                Done := TRUE;
            ELSE
                Error := TRUE;
            END_IF;
        END_IF;
    ELSIF Op = UINT#3 THEN
        IF NOT PendingOpen THEN
            Error := TRUE;
        ELSIF NOT PendingDefChangedEmitted THEN
            Error := TRUE;
        ELSE
            StateEpochId := PendingEpochId;
            EmitSeq := StateSystemSeq;
            StateSystemSeq := StateSystemSeq + ULINT#1;
            EmitSourceTime := StateSourceTime;
            StateSourceTime := StateSourceTime + ULINT#1;
            StartedEncoder(
                RunId := StateRunId,
                Seq := EmitSeq,
                ColdStart := FALSE,
                SourceTime := EmitSourceTime);
            RingProducer(
                Execute := TRUE,
                Record := StartedEncoder.Buffer,
                Length := StartedEncoder.Length,
                RunId := StateRunId,
                SourceId := UDINT#0,
                Seq := EmitSeq);

            IF RingProducer.Done AND (NOT RingProducer.Error) THEN
                StateEpochFirstAbs := RingProducer.LastStartAbs;
                FOR Index := UINT#0 TO UINT#7 DO
                    StatePrevDefinitionHash[Index] := PendingDefHashOld[Index];
                    StateDefinitionHash[Index] := PendingDefHashNew[Index];
                END_FOR;
                PendingOpen := FALSE;
                PendingDefChangedEmitted := FALSE;
                StatePublishedRecordCount := StatePublishedRecordCount + ULINT#1;
                IF EmissionCount < UINT#32 THEN
                    Emissions[EmissionCount].EventTypeId := UDINT#16#0101;
                    Emissions[EmissionCount].RunId := StateRunId;
                    Emissions[EmissionCount].SourceId := UDINT#0;
                    Emissions[EmissionCount].Seq := EmitSeq;
                    EmissionCount := EmissionCount + UINT#1;
                END_IF;
                Done := TRUE;
            ELSE
                Error := TRUE;
            END_IF;
        END_IF;
    ELSIF Op = UINT#4 THEN
        IF NOT PendingOpen THEN
            Error := TRUE;
        ELSIF NOT PendingDefChangedEmitted THEN
            Error := TRUE;
        ELSE
            StateRunId := StateRunId + ULINT#1;
            StateEpochId := PendingEpochId;
            StateSystemSeq := ULINT#0;

            FOR Index := UINT#0 TO UINT#15 DO
                Sources[Index].Used := FALSE;
                Sources[Index].SourceId := UDINT#0;
                Sources[Index].NextSeq := ULINT#0;
            END_FOR;

            EmitSeq := StateSystemSeq;
            StateSystemSeq := StateSystemSeq + ULINT#1;
            EmitSourceTime := StateSourceTime;
            StateSourceTime := StateSourceTime + ULINT#1;
            StartedEncoder(
                RunId := StateRunId,
                Seq := EmitSeq,
                ColdStart := TRUE,
                SourceTime := EmitSourceTime);
            RingProducer(
                Execute := TRUE,
                Record := StartedEncoder.Buffer,
                Length := StartedEncoder.Length,
                RunId := StateRunId,
                SourceId := UDINT#0,
                Seq := EmitSeq);

            IF RingProducer.Done AND (NOT RingProducer.Error) THEN
                StateEpochFirstAbs := RingProducer.LastStartAbs;
                FOR Index := UINT#0 TO UINT#7 DO
                    StatePrevDefinitionHash[Index] := PendingDefHashOld[Index];
                    StateDefinitionHash[Index] := PendingDefHashNew[Index];
                END_FOR;
                PendingOpen := FALSE;
                PendingDefChangedEmitted := FALSE;
                StatePublishedRecordCount := StatePublishedRecordCount + ULINT#1;
                IF EmissionCount < UINT#32 THEN
                    Emissions[EmissionCount].EventTypeId := UDINT#16#0101;
                    Emissions[EmissionCount].RunId := StateRunId;
                    Emissions[EmissionCount].SourceId := UDINT#0;
                    Emissions[EmissionCount].Seq := EmitSeq;
                    EmissionCount := EmissionCount + UINT#1;
                END_IF;
                Done := TRUE;
            ELSE
                Error := TRUE;
            END_IF;
        END_IF;
    ELSE
        Error := TRUE;
    END_IF;
END_IF;

PreviousExecute := Execute;

SourceCount := UINT#0;
FOR Index := UINT#0 TO UINT#15 DO
    SourceUsed[Index] := Sources[Index].Used;
    SourceIds[Index] := Sources[Index].SourceId;
    SourceNextSeqs[Index] := Sources[Index].NextSeq;
    IF Sources[Index].Used THEN
        SourceCount := SourceCount + UINT#1;
    END_IF;
END_FOR;

RunId := StateRunId;
SourceTime := StateSourceTime;
SystemSeq := StateSystemSeq;
PublishedRecordCount := StatePublishedRecordCount;
EpochId := StateEpochId;
EpochFirstAbs := StateEpochFirstAbs;
TransitionOpen := PendingOpen;
DefinitionChangedEmitted := PendingDefChangedEmitted;

FOR Index := UINT#0 TO UINT#7 DO
    DefinitionHash[Index] := StateDefinitionHash[Index];
    PrevDefinitionHash[Index] := StatePrevDefinitionHash[Index];
END_FOR;

IF ExecuteEdge THEN
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
END_IF;
END_FUNCTION_BLOCK
