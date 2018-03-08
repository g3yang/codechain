use kademlia::contact::Contact;
use std::collections::{ BTreeSet, HashMap, VecDeque };

pub struct RoutingTable {
    localhost: Contact,
    buckets: HashMap<usize, Bucket>,
    bucket_size: u8,
}

impl RoutingTable {
    pub fn new(localhost: Contact, bucket_size: u8) -> Self {
        const CAPACITY: usize = 8;
        RoutingTable {
            localhost,
            buckets: HashMap::with_capacity(CAPACITY),
            bucket_size,
        }
    }

    pub fn add_contact(&mut self, contact: Contact) -> Option<&Contact> {
        let index = self.localhost.log2_distance(&contact);
        // FIXME: Decide the maximum distance to contact.
        if index == 0 {
            return None;
        }
        let bucket = self.add_bucket(index);
        bucket.add_contact(contact)
    }

    pub fn remove_contact(&mut self, contact: &Contact) {
        let index = self.localhost.log2_distance(&contact);
        if index == 0 {
            return;
        }

        let bucket = self.buckets.get_mut(&index);
        bucket.map(|bucket| bucket.remove_contact(contact));
    }

    fn add_bucket(&mut self, index: usize) -> &mut Bucket {
        self.buckets.entry(index).or_insert(Bucket::new(self.bucket_size))
    }

    fn remove_bucket(&mut self, index: usize) {
        self.buckets.get(&index)
            .map(|bucket| bucket.is_empty())
            .map(|is_empty| if is_empty {
                self.buckets.remove(&index);
            });
    }

    pub fn get_closest_contacts(&self, target: &Contact) -> Vec<Contact> {
        let contacts = self.get_contacts_in_distance_order(target);
        contacts.into_iter()
            .take(self.bucket_size as usize)
            .map(|item| {
                debug_assert_ne!(target, &item.contact);
                debug_assert_ne!(self.localhost, item.contact);
                item.contact
            })
            .collect()
    }

    fn get_contacts_in_distance_order(&self, target: &Contact) -> BTreeSet<ContactWithDistance> {
        let mut result = BTreeSet::new();
        let mut max_distance = 0;
        for (_, bucket) in self.buckets.iter() {
            for i in 0..self.bucket_size {
                let contact = bucket.contacts.get(i as usize);
                if contact.is_none() {
                    break;
                }

                let contact = contact.unwrap();

                if target == contact {
                    continue;
                }

                let item = ContactWithDistance::new(contact, target);
                if max_distance < item.distance {
                    if (self.bucket_size as usize) <= result.len() {
                        // FIXME: Remove the last item to guarantee the maximum size of return value.
                        continue;
                    }
                    max_distance = item.distance;
                }
                result.insert(item);
            }
        }
        result
    }

    #[cfg(test)]
    fn contains(&self, contact: &Contact) -> bool {
        let index = self.localhost.log2_distance(&contact);
        if index == 0 {
            return false;
        }

        let bucket = self.buckets.get(&index);
        match bucket.map(|bucket| bucket.contains(contact)) {
            None => false,
            Some(has) => has,
        }
    }
}


struct Bucket {
    contacts: VecDeque<Contact>,
    bucket_size: u8,
}

impl Bucket {
    pub fn new(bucket_size: u8) -> Self {
        Bucket {
            contacts: VecDeque::new(),
            bucket_size,
        }
    }

    pub fn add_contact(&mut self, contact: Contact) -> Option<&Contact> {
        self.remove_contact(&contact);
        if self.contacts.iter_mut()
                .find(|old_contact| old_contact.id() == contact.id())
                .is_none() {
            self.contacts.push_back(contact);
        }
        self.head_if_full()
    }

    pub fn remove_contact(&mut self, contact: &Contact) -> bool {
        for i in 0..self.contacts.len() {
            if &self.contacts[i] == contact {
                self.contacts.remove(i);
                return true;
            }
        }
        false
    }

    fn head_if_full(&self) -> Option<&Contact> {
        if self.contacts.len() > self.bucket_size as usize {
            self.contacts.front()
        } else {
            None
        }
    }

    pub fn is_empty(&self) -> bool {
        self.contacts.is_empty()
    }

    #[cfg(test)]
    fn contains(&self, contact: &Contact) -> bool {
        self.contacts.contains(contact)
    }
}


#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ContactWithDistance {
    distance: usize,
    contact: Contact,
}

impl ContactWithDistance {
    pub fn new(contact: &Contact, target: &Contact) -> Self {
        ContactWithDistance {
            distance: contact.log2_distance(&target),
            contact: contact.clone(),
        }
    }
}



#[cfg(test)]
mod tests {
    use std::net::{ IpAddr, Ipv4Addr };
    use std::str::FromStr;
    use super::RoutingTable;
    use super::ContactWithDistance;
    use super::super::contact::Contact;
    use super::super::contact::NodeId;

    const IDS: [&str; 16] = [
        "0000000000000000000000000000000000000000000000000000000000000000\
        0000000000000000000000000000000000000000000000000000000000000000",

        "0000000000000000000000000000000000000000000000000000000000000000\
        0000000000000000000000000000000000000000000000000000000000000001",

        "0000000000000000000000000000000000000000000000000000000000000000\
        0000000000000000000000000000000000000000000000000000000000000010",

        "0000000000000000000000000000000000000000000000000000000000000000\
        0000000000000000000000000000000000000000000000000000000000000011",

        "0000000000000000000000000000000000000000000000000000000000000000\
        0000000000000000000000000000000000000000000000000000000000000100",

        "0000000000000000000000000000000000000000000000000000000000000000\
        0000000000000000000000000000000000000000000000000000000000000101",

        "0000000000000000000000000000000000000000000000000000000000000000\
        0000000000000000000000000000000000000000000000000000000000000110",

        "0000000000000000000000000000000000000000000000000000000000000000\
        0000000000000000000000000000000000000000000000000000000000000111",

        "0000000000000000000000000000000000000000000000000000000000000000\
        0000000000000000000000000000000000000000000000000000000000001000",

        "0000000000000000000000000000000000000000000000000000000000000000\
        0000000000000000000000000000000000000000000000000000000000001001",

        "0000000000000000000000000000000000000000000000000000000000000000\
        0000000000000000000000000000000000000000000000000000000000001010",

        "0000000000000000000000000000000000000000000000000000000000000000\
        0000000000000000000000000000000000000000000000000000000000001011",

        "0000000000000000000000000000000000000000000000000000000000000000\
        0000000000000000000000000000000000000000000000000000000000001100",

        "0000000000000000000000000000000000000000000000000000000000000000\
        0000000000000000000000000000000000000000000000000000000000001101",

        "0000000000000000000000000000000000000000000000000000000000000000\
        0000000000000000000000000000000000000000000000000000000000001110",

        "0000000000000000000000000000000000000000000000000000000000000000\
        0000000000000000000000000000000000000000000000000000000000001111",
    ];

    fn get_contact(distance_from_zero: usize) -> Contact {
        Contact::from_hash(IDS[distance_from_zero])
    }

    fn get_contact_with_address(distance_from_zero: usize, ip: IpAddr, port: u16) -> Contact {
        Contact::from_hash_with_addr(IDS[distance_from_zero], ip, port)
    }

    fn init_routing_table(bucket_size: u8, localhost_index: usize) -> RoutingTable {
        let localhost = get_contact(localhost_index);
        let mut routing_table = RoutingTable::new(localhost, bucket_size);

        for i in 0..IDS.len() {
            if i == localhost_index {
                continue;
            }
            routing_table.add_contact(get_contact(i));
        }
        routing_table
    }

    #[test]
    fn test_size_of_closest_contacts_is_not_larger_than_bucket_size() {
        const BUCKET_SIZE: u8 = 5;
        let mut routing_table = init_routing_table(BUCKET_SIZE, 0);

        let closest_contacts = routing_table.get_closest_contacts(&get_contact(4));
        assert!(closest_contacts.len() <= (BUCKET_SIZE as usize));
    }

    #[test]
    fn test_closest_contacts_1() {
        const BUCKET_SIZE: u8 = 5;
        let mut routing_table = init_routing_table(BUCKET_SIZE, 0);

        let closest_contacts = routing_table.get_closest_contacts(&get_contact(4));
        assert_eq!(BUCKET_SIZE as usize, closest_contacts.len());
        assert_eq!(get_contact(5), closest_contacts[0]);
        assert_eq!(get_contact(6), closest_contacts[1]);
        assert_eq!(get_contact(7), closest_contacts[2]);
        assert_eq!(get_contact(1), closest_contacts[3]);
        assert_eq!(get_contact(2), closest_contacts[4]);
    }

    #[test]
    fn test_closest_contacts_2() {
        const BUCKET_SIZE: u8 = 5;
        let mut routing_table = init_routing_table(BUCKET_SIZE, 0);

        let closest_contacts = routing_table.get_closest_contacts(&get_contact(3));
        assert_eq!(BUCKET_SIZE as usize, closest_contacts.len());
        assert_eq!(get_contact(2), closest_contacts[0]);
        assert_eq!(get_contact(1), closest_contacts[1]);
        assert_eq!(get_contact(4), closest_contacts[2]);
        assert_eq!(get_contact(5), closest_contacts[3]);
        assert_eq!(get_contact(6), closest_contacts[4]);
    }

    #[test]
    fn test_closest_contacts_must_not_contain_target() {
        use std::u8;
        debug_assert!(IDS.len() <= (u8::MAX as usize));
        let bucket_size = IDS.len() as u8;
        let mut routing_table = init_routing_table(bucket_size, 0);

        const TARGET_INDEX: usize = 3;
        let closest_contacts = routing_table.get_closest_contacts(&get_contact(TARGET_INDEX));
        assert!(!closest_contacts.contains(&get_contact(TARGET_INDEX)));
        assert!(2 <= IDS.len());
        let number_of_contacts_except_localhost = IDS.len() - 1;
        let number_of_contacts_except_localhost_and_target = number_of_contacts_except_localhost - 1;
        assert_eq!(number_of_contacts_except_localhost_and_target, closest_contacts.len());
    }

    #[test]
    fn test_add_contact_fails_when_there_is_duplicated_id_with_diffrent_address() {
        use std::u8;
        debug_assert!(IDS.len() <= (u8::MAX as usize));
        let bucket_size = IDS.len() as u8;
        let mut routing_table = init_routing_table(bucket_size, 0);

        let new_contact = get_contact_with_address(4,IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 3485);
        routing_table.add_contact(new_contact.clone());
        let closest_contacts = routing_table.get_closest_contacts(&new_contact);
        assert!(!closest_contacts.contains(&new_contact));
    }
}