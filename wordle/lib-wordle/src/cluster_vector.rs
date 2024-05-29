use std::{collections::HashMap, fmt::{Display, Formatter}};
use crate::{word::Word, parser::Parser, bit_vector_slice::BitVectorSlice};

const BIGGEST_CLUSTER_SHOWN: usize = 5;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct ClusterVector {
    pub value: Vec<usize>
}

impl ClusterVector {
    pub fn new(value: Vec<usize>) -> ClusterVector { 
        ClusterVector { value } 
    }

    pub fn add(&mut self, count: usize) {
        while self.value.len() < count { self.value.push(0); }
        self.value[count - 1] += 1;
    }

    pub fn add_map<T>(&mut self, map: &HashMap<T, Vec<Word>>) {
        for (_, answers) in map.iter() {
            self.add(answers.len());
        }
    }

    pub fn from_map<T>(map: &HashMap<T, Vec<Word>>) -> ClusterVector {
        let mut result = ClusterVector::new(Vec::new());
        result.add_map(map);
        result
    }

    pub fn from_counts<T>(map: &HashMap<T, usize>) -> ClusterVector {
        let mut result = ClusterVector::new(Vec::new());
        result.add_counts(map);
        result
    }

    pub fn from_bits<T>(vec: &Vec<(T, BitVectorSlice)>) -> ClusterVector {
        let mut result = ClusterVector::new(Vec::new());

        for (_, answers) in vec.iter() {
            result.add(answers.count() as usize);
        }

        result
    }

    pub fn add_counts<T>(&mut self, map: &HashMap<T, usize>) {
        for (_, count) in map.iter() {
            self.add(*count);
        }
    }

    pub fn clear(&mut self) {
        self.value.clear();
    }

    pub fn cluster_count(&self) -> usize {
        self.value.iter().sum()
    }

    pub fn word_count(&self) -> usize {
        self.value.iter().enumerate().map(|(i, count)| count * (i + 1)).sum()
    }

    pub fn biggest_cluster(&self) -> usize {
        let mut biggest = 0;
        
        for (i, count) in self.value.iter().enumerate() {
            if *count > 0 { biggest = i + 1; }
        }

        biggest
    }

    pub fn to_string(&self) -> String {
        let mut result = String::new();
        let cluster_count = self.cluster_count();
        let biggest_cluster = self.biggest_cluster();

        if cluster_count == 0 {
            return result;
        }
    
        result += "[";
    
        // Always show first cluster counts
        for (i, count) in self.value.iter().enumerate() {
            if i >= biggest_cluster { break; }
            if i >= BIGGEST_CLUSTER_SHOWN { break; }
            if i > 0 { result += ", "; }
            result += &format!("{}", count);
        }
    
        // Show largest cluster size, if too many to show all
        if biggest_cluster > BIGGEST_CLUSTER_SHOWN {
            result += &format!(" .. ^{biggest_cluster}");
        }
    
        // Show total number of clusters, if enough to warrant
        if cluster_count > 20 && biggest_cluster > 1 {
            if biggest_cluster <= BIGGEST_CLUSTER_SHOWN {
                result += " ..";
            }

            result += &format!(" ∑{cluster_count}")
        }
    
        result += "]";
    
        result
    }

    pub fn parse(parser: &mut Parser) -> Result<Option<ClusterVector>, String> {
        parser.require("[")?;
        let mut values = Vec::new();

        while parser.current != "]" {
            // If ".." found, the complete vector wasn't written, so we can't reconstruct it safely.
            if parser.current == ".." {
                while parser.current != "]" {
                    parser.next()?;
                }

                parser.require("]")?;
                return Ok(None);
            }

            values.push(parser.as_usize()?);
            parser.next()?;

            // Require commas between values
            if parser.current != "]" && parser.current != ".." {
                parser.require(",")?;
            }
        }

        parser.require("]")?;
        Ok(Some(ClusterVector::new(values)))
    }

    pub fn total_turns_ideal(&self) -> usize {
        let mut answer_count = 0;
        let mut cluster_count = 0;
    
        // Assume that every answer in every cluster will be found in two turns,
        // except one answer in each cluster which is guessed first and will be found in one turn.
        for (i, count) in self.value.iter().enumerate() {
            let n = i + 1;
            cluster_count += count;
            answer_count += count * n;
        }
    
        (2 * answer_count) - cluster_count
    }

    pub fn total_turns_pessimistic(&self) -> usize {
        let mut total = 0;
    
        // Each 1-cluster: 1 turn.
        // Each 2-cluster: 1 + 2 = 3 turns.
        // Each 3-cluster: 1 + 2 + 3 = 6 turns.
        // In general, each cluster takes (n * (n+1) / 2) turns.
        // If there are 'count' clusters of size n, then altogether they take (n * (n+1) / 2) * count turns.
    
        for (i, count) in self.value.iter().enumerate() {
            let n = i + 1;
            total += (n) * (n + 1) * count;
        }
    
        total / 2
    }

    pub fn total_turns_predicted(&self) -> usize {
        let mut double_total = 0;

        // Each 1-cluster: 1 turn.
        // Each 2-cluster: 1 + 2 = 3 turns.
        // Each 3-cluster: 1 + 2 + 2 = 5 or 1 + 2 + 3 = 6 turns, about 50/50. => 5.5 turns.
        // Larger N-cluster: 2.5 * N (50/50 one guess or two and then it's a single)
        
        // Compute double the expected and then divide by two to handle '5.5' in a usize.
        for (i, count) in self.value.iter().enumerate() {
            let n = i + 1;
            
            match n {
                1 => double_total += 2 * count,
                2 => double_total += 6 * count,
                3 => double_total += 11 * count,
                _ => double_total += 4 * n * count,
            }
        }

        double_total / 2
    }
}

impl Display for ClusterVector {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basics() {
        let mut cv = ClusterVector::new(vec![1, 2, 3, 4, 5, 0, 0]);
        assert_eq!(cv.cluster_count(), 15);
        assert_eq!(cv.word_count(), 1 + 2*2 + 3*3 + 4*4 + 5*5);
        assert_eq!(cv.biggest_cluster(), 5);
        assert_eq!(cv.to_string(), "[1, 2, 3, 4, 5]");

        cv.clear();
        assert_eq!(cv.cluster_count(), 0);
        assert_eq!(cv.word_count(), 0);

        // Add a map with a 2- and 3- cluster
        let mut map = HashMap::new();
        map.insert(0, vec![w("aaaaa"), w("bbbbb")]);
        map.insert(1, vec![w("aaaaa"), w("bbbbb"), w("ccccc")]);
        
        cv.add_map(&map);
        assert_eq!(cv.cluster_count(), 2);
        assert_eq!(cv.word_count(), 5);
        assert_eq!(cv.biggest_cluster(), 3);
        assert_eq!(cv.to_string(), "[0, 1, 1]");

        // Add a single count for a 2-cluster 
        //  (each key is a cluster and the value is the number of words which would've been in it)
        let mut counts = HashMap::new();
        counts.insert(0, 2);
        cv.add_counts(&counts);
        assert_eq!(cv.cluster_count(), 3);
        assert_eq!(cv.to_string(), "[0, 2, 1]");

    }

    #[test]
    fn total_turns() {
        // Pessimistic
        // ===========

        // 6 1-clusters: 6 turns
        assert_eq!(ClusterVector::new(vec![6]).total_turns_pessimistic(), 6);

        // 4 1-clusters, 1 2-cluster: 4 + (1 + 2) = 7 turns
        assert_eq!(ClusterVector::new(vec![4, 1]).total_turns_pessimistic(), 7);

        // 3 1-clusters, 1 3-cluster: 3 + (1 + 2 + 3) = 9 turns
        assert_eq!(ClusterVector::new(vec![3, 0, 1]).total_turns_pessimistic(), 9);


        // Ideal
        // =====

        // 6 1-clusters: 6 turns
        assert_eq!(ClusterVector::new(vec![6]).total_turns_ideal(), 6);

        // 4 1-clusters, 1 2-cluster: 4 + (1 + 2) = 7 turns
        assert_eq!(ClusterVector::new(vec![4, 1]).total_turns_ideal(), 7);

        // 3 1-clusters, 1 3-cluster: 3 + (1 + 2 + 2) = 8 turns
        assert_eq!(ClusterVector::new(vec![3, 0, 1]).total_turns_ideal(), 8);

        // 10 3-clusters: 30 answers * 2 turns - 10 cluster first guesses = 50
        assert_eq!(ClusterVector::new(vec![0, 0, 10]).total_turns_ideal(), 50);
    }

    #[test]
    fn print_cluster_vector() {
        // No clusters -> ""
        assert_eq!(&ClusterVector::new(vec![0, 0, 0, 0]).to_string(), "");

        // No commas and no count for all 1-clusters
        assert_eq!(&ClusterVector::new(vec![23]).to_string(), "[23]");

        // Show surrounded zero elements. Omit total count when under 10.
        assert_eq!(&ClusterVector::new(vec![4, 0, 0, 1]).to_string(), "[4, 0, 0, 1]");

        // Don't show zeros above last non-zero
        assert_eq!(&ClusterVector::new(vec![4, 0, 0, 1, 0, 0, 0, 0]).to_string(), "[4, 0, 0, 1]");

        // Show all buckets under 6-clusters (cv[5])
        let cv = ClusterVector::new(vec![2104, 85, 11, 2, 1]);
        assert_eq!(cv.to_string(), format!("[2104, 85, 11, 2, 1 .. ∑{}]", cv.value.iter().sum::<usize>()));

        // Summarize buckets above 6-clusters. Show correct count of remaining clusters. Show largest cluster (by element count)
        let cv = ClusterVector::new(vec![632, 202, 105, 43, 30, 18, 18, 3, 7, 4, 4, 1]);
        let count = cv.value.iter().sum::<usize>();
        assert_eq!(cv.to_string(), format!("[632, 202, 105, 43, 30 .. ^12 ∑{}]", count));

        // Omit "+big" when only one big cluster. Don't show count under 20.
        let cv = ClusterVector::new(vec![5, 4, 3, 2, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
        assert_eq!(cv.to_string(), "[5, 4, 3, 2, 1 .. ^14]");
    }

    #[test]
    fn parse_cluster_vector() {
        // All singles
        roundtrip(ClusterVector::new(vec![2315]), true);

        // Small enough for no summary
        roundtrip(ClusterVector::new(vec![6, 4, 1]), true);

        // Summary required; CV should not roundtrip
        roundtrip(ClusterVector::new(vec![632, 202, 105, 43, 30, 18, 18, 3, 7, 4, 4, 1]), false);
    }

    fn roundtrip(cv: ClusterVector, should_parse: bool) -> Option<ClusterVector> {
        // Convert to string
        let text = cv.to_string();

        // Parse back
        let mut parser = Parser::new(text.lines());
        let parsed = ClusterVector::parse(&mut parser);

        // Verify parse reports no error
        assert!(parsed.is_ok());

        // Verify parser has no more tokens
        assert!(parser.next().is_err());

        // If the CV was not summarized, verify is is equal
        let parsed = parsed.unwrap();
        if text.contains("..") {
            assert_eq!(parsed, None);
        } else {
            assert_eq!(parsed.as_ref().unwrap(), &cv);
        }

        // Verify it parsed or not as expected
        assert_eq!(parsed.as_ref().is_some(), should_parse);

        parsed
    }

    fn w(text: &str) -> Word {
        Word::new(text).unwrap()
    }
}