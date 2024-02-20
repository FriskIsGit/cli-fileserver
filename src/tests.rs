use crate::packet::{FIELD_OFFSET, TransferOfferPacket, FilePacket, Packet, AnswerPacket};

#[test]
fn file_packet_test() {
    let original = FilePacket::new(0, 9, vec![1, 2, 3, 4, 5, 6, 7, 8, 9]);
    let parcel = original.parcel();
    let field_bytes = &parcel[FIELD_OFFSET..parcel.len()];
    let constructed = FilePacket::construct_packet(field_bytes).expect("Failed to construct FilePacket packet");
    assert_eq!(original.chunk_id, constructed.chunk_id);
    assert_eq!(original.payload_size, constructed.payload_size);
    assert_eq!(original.file_bytes, constructed.file_bytes);
}
#[test]
fn transfer_offer_test() {
    let original = TransferOfferPacket::new(313, "àáąâãäå.zip".into());
    let parcel = original.parcel();
    let field_bytes = &parcel[FIELD_OFFSET..parcel.len()];
    let constructed = TransferOfferPacket::construct_packet(field_bytes).expect("Failed to construct FileInfoPacket packet");
    assert_eq!(original.file_size, constructed.file_size);
    assert_eq!(original.file_name, constructed.file_name);
}
#[test]
fn response_packet_test() {
    let original_true = AnswerPacket::new(true);
    let original_false = AnswerPacket::new(false);

    let true_parcel = original_true.parcel();
    let false_parcel = original_false.parcel();

    let true_field_byte = &true_parcel[FIELD_OFFSET..true_parcel.len()];
    let false_field_byte = &false_parcel[FIELD_OFFSET..false_parcel.len()];

    let true_constructed = AnswerPacket::construct_packet(true_field_byte).expect("Failed to construct FileInfoPacket packet");
    let false_constructed = AnswerPacket::construct_packet(false_field_byte).expect("Failed to construct FileInfoPacket packet");
    assert_eq!(original_true.yes(), true_constructed.yes());
    assert_eq!(original_false.yes(), false_constructed.yes());
}